/// CLI Command: `adopt service <artifact_id> [scope] <services...>`
///
/// What it does:
/// Records systemd service unit names in an artifact's `[bin].toml` under `[services.user]` or `[services.system]`.
///
/// Variations:
/// 1. `scope` provided: Uses explicit scope (`user` or `system`).
/// 2. `scope` omitted: Prompts interactively when in TUI mode or defaults to `user` scope.
///
/// Decisions & Logic Branches:
/// - Verifies that the target artifact directory exists in the workspace.
/// - Appends service unit names to `[services.user]` or `[services.system]` array without duplicate entries.
/// - Automatically appends `.service` suffix if unit name omits an extension.
/// - Ensures `[about].toml` contains an entry for the artifact.
use color_eyre::eyre::{Result, bail};

use crate::commands::lib::{ensure_about_entry, repository_path, split_artifact_id};
use crate::types::{BIN_TOML, BinFile, Runtime};
use crate::utils::style;

fn prompt_scope(runtime: &Runtime, scope: Option<String>) -> Result<String> {
    if let Some(s) = scope {
        let s_lower = s.trim().to_ascii_lowercase();
        if s_lower != "user" && s_lower != "system" {
            bail!("invalid service scope '{}', must be 'user' or 'system'", s);
        }
        Ok(s_lower)
    } else if !runtime.no_color {
        let choice = cliclack::select("Service Scope")
            .item("user".to_string(), "User service (systemctl --user)", "")
            .item("system".to_string(), "System service (systemctl)", "")
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;
        Ok(choice)
    } else {
        Ok("user".to_string())
    }
}

fn prompt_services(runtime: &Runtime, services: Vec<String>) -> Result<Vec<String>> {
    if !services.is_empty() {
        Ok(services)
    } else if !runtime.no_color {
        let input: String = cliclack::input("Enter service unit name(s) (space separated):")
            .placeholder("syncthing.service")
            .interact::<String>()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;
        let units: Vec<String> = input
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if units.is_empty() {
            bail!("The adopt service command requires at least one service unit name.");
        }
        Ok(units)
    } else {
        bail!(
            "The adopt service command requires service unit names when running non-interactively."
        );
    }
}

pub fn run(
    runtime: &Runtime,
    artifact_id: &str,
    scope_or_service: Option<String>,
    mut services: Vec<String>,
) -> Result<()> {
    let (repo, artifact) = split_artifact_id(artifact_id)?;

    let artifact_dir = repository_path(runtime, repo).join(artifact);
    if !artifact_dir.exists() {
        bail!(
            "artifact directory does not exist: {}",
            artifact_dir.display()
        );
    }

    let (scope_arg, service_units) = match scope_or_service {
        Some(s) if s.eq_ignore_ascii_case("user") || s.eq_ignore_ascii_case("system") => {
            (Some(s), prompt_services(runtime, services)?)
        }
        Some(first_svc) => {
            services.insert(0, first_svc);
            (None, prompt_services(runtime, services)?)
        }
        None => (None, prompt_services(runtime, services)?),
    };

    let scope_name = prompt_scope(runtime, scope_arg)?;

    let bin_path = artifact_dir.join(BIN_TOML);
    let mut bin_file: BinFile = if bin_path.exists() {
        crate::types::read_toml(&bin_path)?
    } else {
        BinFile::default()
    };

    let set = bin_file.services.entry(scope_name.clone()).or_default();
    let mut added = Vec::new();
    for unit in service_units {
        let formatted = if unit.contains('.') {
            unit
        } else {
            format!("{unit}.service")
        };
        if !set.units.contains(&formatted) {
            set.units.push(formatted.clone());
            added.push(formatted);
        }
    }

    crate::types::write_toml(&bin_path, &bin_file)?;
    ensure_about_entry(runtime, repo, artifact)?;

    for unit in added {
        println!(
            "Added ({}) service {} to {} [bin].toml",
            scope_name,
            style(&unit, "32", runtime),
            style(artifact_id, "36;1", runtime)
        );
    }

    Ok(())
}
