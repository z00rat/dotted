use clap::Parser;
use color_eyre::Result;

pub(crate) mod cli;
pub(crate) mod commands;
pub(crate) mod plan;
pub(crate) mod types;
pub(crate) mod utils;

use cli::{
    AdoptCommands, ArtifactCommands, BackupCommands, Cli, Commands, DeployCommands, FilesCommands,
    IgnoreCommands, RepoCommands, ShellCommands, WorkspaceCommands,
};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    run_with_cli(cli)
}

#[allow(clippy::too_many_lines)]
fn run_with_cli(cli: Cli) -> Result<()> {
    let runtime = types::Runtime::from_cli(&cli)?;
    let is_init_or_completions = matches!(
        cli.command,
        Commands::Workspace(WorkspaceCommands::Init { .. })
            | Commands::Shell(ShellCommands::Completions { .. })
    );
    if !is_init_or_completions && !runtime.dotted_dir.exists() {
        color_eyre::eyre::bail!(
            "dotted directory does not exist at {}. Please run `dotted workspace init` first.",
            runtime.dotted_dir.display()
        );
    }
    match cli.command {
        Commands::Workspace(sub) => match sub {
            WorkspaceCommands::Init { git_url } => {
                commands::workspace::init::run(&runtime, git_url)
            }
            WorkspaceCommands::Pull => commands::workspace::pull::run(&runtime),
            WorkspaceCommands::Push => commands::workspace::push::run(&runtime),
            WorkspaceCommands::Cd => commands::workspace::cd::run(&runtime),
            WorkspaceCommands::Doctor { category } => {
                commands::workspace::doctor::run(&runtime, category.as_deref())
            }
        },
        Commands::Artifact(sub) => match sub {
            ArtifactCommands::List { filter, raw, state } => {
                commands::artifact::list::run(&runtime, filter.as_deref(), raw, state.as_deref())
            }
            ArtifactCommands::Show { artifact } => {
                commands::artifact::show::run(&runtime, &artifact)
            }
            ArtifactCommands::Create { artifact_name } => {
                commands::artifact::create::run(&runtime, &artifact_name)
            }
            ArtifactCommands::Enable { artifact } => {
                commands::artifact::enable::run(&runtime, &artifact)
            }
            ArtifactCommands::Disable { artifact } => {
                commands::artifact::disable::run(&runtime, &artifact)
            }
            ArtifactCommands::Uninstall { artifact, yes } => {
                commands::artifact::uninstall::run(&runtime, &artifact, yes)
            }
        },
        Commands::Adopt(sub) => match sub {
            AdoptCommands::File { artifact, path } => {
                commands::adopt::file::run(&runtime, &artifact, path)
            }
            AdoptCommands::Package {
                artifact,
                package,
                package_type,
            } => commands::adopt::package::run(&runtime, &artifact, package, package_type),
        },
        Commands::Deploy(sub) => match sub {
            DeployCommands::Status { artifact, filter } => {
                commands::deploy::status::run(&runtime, artifact.as_deref(), filter.as_deref())
            }
            DeployCommands::Diff { artifact } => {
                commands::deploy::diff::run(&runtime, artifact.as_deref(), None)
            }
            DeployCommands::Apply(args) => commands::deploy::apply::run(&runtime, &args),
            DeployCommands::Orphans { filter } => {
                commands::deploy::orphans::run(&runtime, filter.as_deref())
            }
        },
        Commands::Repo(sub) => match sub {
            RepoCommands::List => commands::repo::list::run(&runtime),
            RepoCommands::Add { name, git_url } => {
                commands::repo::add::run(&runtime, &name, &git_url)
            }
            RepoCommands::Remove { name } => commands::repo::remove::run(&runtime, &name),
            RepoCommands::About { name } => commands::repo::about::run(&runtime, &name),
        },
        Commands::Files(sub) => match sub {
            FilesCommands::List(args) => commands::files::list::run(&runtime, &args),
            FilesCommands::Scan { path, filter } => {
                commands::files::scan::run(&runtime, path, filter)
            }
            FilesCommands::Ignore(ignore_sub) => match ignore_sub {
                IgnoreCommands::Add { path } => {
                    commands::files::ignore::add(&runtime, path.as_deref())
                }
                IgnoreCommands::Remove { path } => {
                    commands::files::ignore::remove(&runtime, path.as_deref())
                }
            },
        },
        Commands::Backup(sub) => match sub {
            BackupCommands::List { filter } => {
                commands::backup::list::run(&runtime, None, filter.as_deref())
            }
            BackupCommands::Restore { timestamp, path } => {
                commands::backup::restore::run(&runtime, &timestamp, path.as_deref())
            }
        },
        Commands::Shell(sub) => match sub {
            ShellCommands::Env { shell } => commands::shell::env::run(&runtime, shell),
            ShellCommands::Completions { shell } => commands::shell::completions::run(shell),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn runtime() -> (TempDir, types::Runtime) {
        let temp = TempDir::new().expect("temp dir");
        let runtime = types::Runtime {
            dotted_dir: temp.path().join("meta"),
            home_dir: temp.path().join("home"),
            root_dir: temp.path().join("root"),
            user: "user1".to_string(),
            device: "laptop".to_string(),
            distro: "archlinux".to_string(),
            no_color: false,
        };
        (temp, runtime)
    }

    #[test]
    fn replacements_apply_only_to_text() {
        let mut replace = BTreeMap::new();
        replace.insert("NAME".to_string(), "user".to_string());
        assert_eq!(
            plan::text_with_replace(b"hello NAME", &replace),
            Some("hello user".to_string())
        );
        assert_eq!(plan::text_with_replace(b"hello\0NAME", &replace), None);
    }

    #[test]
    fn maps_home_and_system_paths_to_safe_root() {
        let (_temp, runtime) = runtime();
        let (target, display) =
            plan::map_artifact_path(&runtime, Path::new("home/.config/a")).unwrap();
        assert_eq!(target, runtime.home_dir.join(".config/a"));
        assert_eq!(display, runtime.home_dir.join(".config/a"));

        let (target, display) =
            plan::map_artifact_path(&runtime, Path::new("etc/app.conf")).unwrap();
        assert_eq!(target, runtime.root_dir.join("etc/app.conf"));
        assert_eq!(display, PathBuf::from("/etc/app.conf"));
    }

    #[test]
    fn matches_any_glob_behavior() {
        let mut patterns = BTreeSet::new();
        patterns.insert(PathBuf::from("/home/user/.config/app/log*.jsonl"));
        patterns.insert(PathBuf::from("/home/user/.cache"));

        // Helper function uses globe match
        let matches = |path: &Path, pat: &BTreeSet<PathBuf>| -> bool {
            let path_str = path.to_string_lossy();
            for pattern in pat {
                let pattern_str = pattern.to_string_lossy();
                if glob::Pattern::new(&pattern_str).is_ok_and(|p| p.matches(&path_str)) {
                    return true;
                }
            }
            false
        };

        assert!(matches(
            Path::new("/home/user/.config/app/log123.jsonl"),
            &patterns
        ));
        assert!(matches(Path::new("/home/user/.cache"), &patterns));
        assert!(!matches(
            Path::new("/home/user/.config/app/log123.txt"),
            &patterns
        ));
    }

    #[test]
    fn terminal_color_names_are_validated() {
        assert!(utils::is_terminal_color("bright-yellow"));
        assert!(utils::is_terminal_color("cyan"));
        assert!(!utils::is_terminal_color("orange"));
    }

    #[test]
    fn display_paths_are_consistent() {
        let (_temp, runtime) = runtime();
        assert_eq!(
            runtime.display_path(&runtime.home_dir.join(".config/fish")),
            PathBuf::from("~/.config/fish")
        );
        assert_eq!(
            runtime.display_path(&runtime.dotted_dir.join("[artifacts]/shell")),
            PathBuf::from("artifacts/shell")
        );
    }
}
