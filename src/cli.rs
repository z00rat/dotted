use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use std::io;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "dotted",
    version,
    about = "A simple, templateless, multi-[device|repo|user|distro] dotfile manager that is highly shareable and tracks system packages & services."
)]
pub struct Cli {
    #[arg(long, global = true, hide = true, env = "DOTTED_DIR")]
    pub(crate) dotted_dir: Option<PathBuf>,
    #[arg(long, global = true, hide = true, env = "DOTTED_HOME_DIR")]
    pub(crate) home_dir: Option<PathBuf>,
    #[arg(long, global = true, hide = true, env = "DOTTED_ROOT_DIR")]
    pub(crate) root_dir: Option<PathBuf>,
    #[arg(long, global = true, help = "Override device name")]
    pub(crate) device: Option<String>,
    #[arg(long, global = true, help = "Override resolved user")]
    pub(crate) user: Option<String>,
    #[arg(long, global = true, value_parser = ["archlinux", "fedora", "ubuntu"], help = "Override detected Linux distribution")]
    pub(crate) distro: Option<String>,
    #[arg(long, global = true, help = "Disable colored output")]
    pub(crate) no_color: bool,
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    #[command(subcommand, about = "Workspace management")]
    Workspace(WorkspaceCommands),
    #[command(subcommand, about = "Artifact management")]
    Artifact(ArtifactCommands),
    #[command(subcommand, about = "Adopt system files or packages")]
    Adopt(AdoptCommands),
    #[command(subcommand, about = "Deploy management")]
    Deploy(DeployCommands),
    #[command(subcommand, about = "Repository management")]
    Repo(RepoCommands),
    #[command(subcommand, about = "Ignore rules management")]
    Ignore(IgnoreCommands),
    #[command(subcommand, about = "Backup & restore management")]
    Backup(BackupCommands),
    #[command(subcommand, about = "Shell environments & completions")]
    Shell(ShellCommands),
}

#[derive(Debug, Subcommand)]
pub(crate) enum WorkspaceCommands {
    #[command(about = "Initialize the local workspace")]
    Init {
        #[arg(help = "Optional git URL to clone from")]
        git_url: Option<String>,
    },
    #[command(about = "Pull and sync all remote repositories")]
    Pull,
    #[command(about = "Commit and push local changes")]
    Push,
    #[command(about = "Open a shell in the dotted workspace")]
    Cd,
    #[command(about = "Check workspace and system configuration health")]
    Doctor {
        #[arg(value_parser = ["config", "repo", "artifact", "tool"], help = "Check category")]
        category: Option<String>,
    },
    #[command(about = "Format all dotted TOML configuration files in the workspace")]
    Format,
}

#[derive(Debug, Subcommand)]
pub(crate) enum ArtifactCommands {
    #[command(about = "List all discovered artifacts")]
    List {
        #[arg(short, long, help = "Filter results by repo, name or description")]
        filter: Option<String>,
        #[arg(long, hide = true)]
        raw: bool,
        #[arg(long, hide = true, value_parser = ["enabled", "disabled"])]
        state: Option<String>,
    },
    #[command(about = "Show details of a specific artifact")]
    Show {
        #[arg(help = "Artifact ID (repo/artifact) to show details for")]
        artifact: String,
    },
    #[command(about = "Create a new artifact scaffold")]
    Create {
        #[arg(help = "Name of the new artifact to scaffold")]
        artifact: String,
    },
    #[command(about = "Enable one or more artifacts")]
    Enable {
        #[arg(help = "Artifact ID(s) (repo/artifact) to enable", num_args = 1..)]
        artifacts: Vec<String>,
    },
    #[command(about = "Disable one or more artifacts")]
    Disable {
        #[arg(help = "Artifact ID(s) (repo/artifact) to disable", num_args = 1..)]
        artifacts: Vec<String>,
    },
    #[command(about = "Uninstall and disable an artifact")]
    Uninstall {
        #[arg(help = "Artifact ID (repo/artifact) to remove")]
        artifact: String,
        #[arg(short, long, help = "Skip confirmation prompts")]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum AdoptCommands {
    #[command(about = "Adopt system files or directories into your personal repository")]
    File {
        #[arg(help = "Artifact ID (repo/artifact) to adopt into")]
        artifact: String,
        #[arg(help = "Path(s) to the system file/directory to adopt", num_args = 1..)]
        paths: Vec<PathBuf>,
    },
    #[command(about = "Adopt a package manager dependency")]
    Package {
        #[arg(help = "Artifact ID (repo/artifact) to adopt package into")]
        artifact: String,
        #[arg(help = "Name of the package to add")]
        package: Option<String>,
        #[arg(long = "type", value_parser = ["archlinux", "fedora", "ubuntu", "flatpak"], help = "Package type")]
        package_type: Option<String>,
    },
    #[command(about = "Adopt a systemd service unit")]
    Service {
        #[arg(help = "Artifact ID (repo/artifact) to adopt service into")]
        artifact: String,
        #[arg(help = "Service scope ('user' or 'system') or service unit name")]
        scope_or_service: Option<String>,
        #[arg(help = "Additional service unit name(s) to add")]
        services: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum DeployCommands {
    #[command(about = "Show deployment preview and status")]
    Status {
        #[arg(help = "Filter status to a specific artifact")]
        artifact: Option<String>,
        #[arg(short, long, value_parser = ["artifacts", "files", "env", "packages", "downloads", "services"], help = "Status category")]
        filter: Option<String>,
    },
    #[command(about = "Show differences between rendered files and active system files")]
    Diff {
        #[arg(help = "Show diff for a specific artifact only")]
        artifact: Option<String>,
    },
    #[command(about = "Apply changes to system files and install dependencies")]
    Apply(ApplyArgs),
    #[command(about = "List packages, downloads, and services not declared by active artifacts")]
    Orphans {
        #[arg(short, long, value_parser = ["native", "flatpak", "downloads", "services"], help = "Category")]
        filter: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum RepoCommands {
    #[command(about = "List configured repositories")]
    List,
    #[command(about = "Add a remote repository to workspace")]
    Add {
        #[arg(help = "Name of the repository")]
        name: String,
        #[arg(help = "Git clone URL")]
        git_url: String,
    },
    #[command(about = "Remove a repository configuration")]
    Remove {
        #[arg(help = "Name of the repository to remove")]
        name: String,
    },
    #[command(about = "Show metadata of a repository")]
    About {
        #[arg(help = "Name of the repository to show about info for")]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum IgnoreCommands {
    #[command(subcommand, about = "Manage ignored files and directories")]
    File(IgnoreFileCommands),
    #[command(subcommand, about = "Manage ignored system packages")]
    Package(IgnorePackageCommands),
    #[command(subcommand, about = "Manage ignored systemd services")]
    Service(IgnoreServiceCommands),
}

#[derive(Debug, Subcommand)]
pub(crate) enum IgnoreFileCommands {
    #[command(about = "Add one or more file or directory paths to ignore list")]
    Add {
        #[arg(help = "Path(s) to add to settings [ignore]")]
        paths: Vec<PathBuf>,
    },
    #[command(about = "Remove one or more file or directory paths from ignore list")]
    Remove {
        #[arg(help = "Path(s) to remove from settings [ignore]")]
        paths: Vec<PathBuf>,
    },
    #[command(name = "list", about = "List tracked, untracked, and ignored files")]
    List(LsArgs),
    #[command(
        name = "scan",
        about = "Scan filesystem paths recursively for tracked, untracked, and ignored files"
    )]
    Scan {
        #[arg(long, help = "Explicit root directory to scan")]
        path: Option<PathBuf>,
        #[arg(short, long, value_parser = ["tracked", "untracked", "ignored", "partial", "masked"], help = "File status category")]
        filter: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum IgnorePackageCommands {
    #[command(about = "Add one or more system packages to ignore list")]
    Add {
        #[arg(help = "Package name(s) to add to settings [ignore]")]
        names: Vec<String>,
    },
    #[command(about = "Remove one or more system packages from ignore list")]
    Remove {
        #[arg(help = "Package name(s) to remove from settings [ignore]")]
        names: Vec<String>,
    },
    #[command(about = "List ignored system packages")]
    List,
}

#[derive(Debug, Subcommand)]
pub(crate) enum IgnoreServiceCommands {
    #[command(about = "Add one or more systemd service units to ignore list")]
    Add {
        #[arg(help = "Service unit name(s) to add to settings [ignore]")]
        names: Vec<String>,
    },
    #[command(about = "Remove one or more systemd service units from ignore list")]
    Remove {
        #[arg(help = "Service unit name(s) to remove from settings [ignore]")]
        names: Vec<String>,
    },
    #[command(about = "List ignored systemd services")]
    List,
}

#[derive(Debug, Subcommand)]
pub(crate) enum BackupCommands {
    #[command(about = "List backup snapshots")]
    List {
        #[arg(short, long, help = "Optional path filter")]
        filter: Option<PathBuf>,
    },
    #[command(about = "Restore files from a backup snapshot")]
    Restore {
        #[arg(help = "Timestamp of the backup run to restore")]
        timestamp: String,
        #[arg(help = "Optional single path to restore")]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ShellCommands {
    #[command(about = "Print environment variables for active shell integration")]
    Env {
        #[arg(short, long, help = "Shell syntax to output (bash, zsh, fish)")]
        shell: Option<Shell>,
    },
    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(help = "Target shell for completion generation")]
        shell: Shell,
    },
}

#[derive(Debug, Args)]
pub(crate) struct ApplyArgs {
    #[arg(help = "Apply changes only from the specified artifact")]
    pub(crate) artifact: Option<String>,
    #[arg(short, long, help = "Skip confirmation prompts")]
    pub(crate) yes: bool,
}

#[derive(Debug, Args)]
pub(crate) struct LsArgs {
    #[arg(long, help = "Depth of directory traversal (0 for unlimited)")]
    pub(crate) depth: Option<usize>,
    #[arg(long, help = "Explicit root directory to scan")]
    pub(crate) path: Option<PathBuf>,
    #[arg(short, long, value_parser = ["tracked", "untracked", "ignored", "partial", "masked"], help = "File status category")]
    pub(crate) filter: Option<String>,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn completions(shell: Shell) -> color_eyre::Result<()> {
    use std::io::Write;
    let mut command = Cli::command();
    if shell == Shell::Fish {
        let mut buffer = Vec::new();
        generate(shell, &mut command, "dotted", &mut buffer);
        let mut output = String::from_utf8(buffer)?;
        output.push_str("\n# Dynamic completions for artifacts and repos\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from enable' -a '(dotted artifact list --raw --state disabled 2>/dev/null)'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from disable' -a '(dotted artifact list --raw --state enabled 2>/dev/null)'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from uninstall status diff apply show package service' -a '(dotted artifact list --raw 2>/dev/null)'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from file; and test (count (commandline -opc)) -eq 3' -a '(dotted artifact list --raw 2>/dev/null)'\n");
        output.push_str("complete -c dotted -F -n '__fish_seen_subcommand_from file; and test (count (commandline -opc)) -ge 4' -a '(__fish_complete_path)'\n");
        output.push_str("complete -c dotted -F -n '__fish_seen_subcommand_from ignore; and __fish_seen_subcommand_from file'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from ignore' -n 'not __fish_seen_subcommand_from file package service' -a 'file package service'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from ignore; and __fish_seen_subcommand_from file' -n 'not __fish_seen_subcommand_from add remove list scan' -a 'add remove list scan'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from ignore; and __fish_seen_subcommand_from package' -n 'not __fish_seen_subcommand_from add remove list' -a 'add remove list'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from ignore; and __fish_seen_subcommand_from service' -n 'not __fish_seen_subcommand_from add remove list' -a 'add remove list'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from repo; and __fish_seen_subcommand_from remove about' -a '(dotted repo list 2>/dev/null | tail -n +4 | head -n -1 | awk \"{print \\$2}\")'\n");
        output.push_str("complete -c dotted -f -n '__fish_seen_subcommand_from artifact; and __fish_seen_subcommand_from create' -a '(dotted repo list 2>/dev/null | tail -n +4 | head -n -1 | awk \"{print \\$2 \\\"/\\\"}\")'\n");
        io::stdout().write_all(output.as_bytes())?;
    } else if shell == Shell::Bash {
        let mut buffer = Vec::new();
        generate(shell, &mut command, "dotted", &mut buffer);
        let mut output = String::from_utf8(buffer)?;
        output = output.replacen("_dotted() {", "_dotted_generated() {", 1);
        for target in &[
            "dotted__subcmd__artifact__subcmd__enable",
            "dotted__subcmd__artifact__subcmd__disable",
            "dotted__subcmd__artifact__subcmd__uninstall",
            "dotted__subcmd__artifact__subcmd__show",
            "dotted__subcmd__deploy__subcmd__apply",
            "dotted__subcmd__deploy__subcmd__diff",
            "dotted__subcmd__deploy__subcmd__status",
            "dotted__subcmd__adopt__subcmd__package",
            "dotted__subcmd__adopt__subcmd__service",
        ] {
            let pattern = format!("{target})\n            opts=\"\"");
            let state = if target.ends_with("__enable") {
                "--state disabled"
            } else if target.ends_with("__disable") {
                "--state enabled"
            } else {
                ""
            };
            let replacement = format!(
                "{target})\n            opts=\"$(dotted artifact list --raw {state} 2>/dev/null)\""
            );
            output = output.replace(&pattern, &replacement);
        }
        let file_target = "dotted__subcmd__adopt__subcmd__file";
        let pattern = format!("{file_target})\n            opts=\"\"");
        let replacement = format!(
            "{file_target})\n            local file_idx=-1\n            for i in \"${{!COMP_WORDS[@]}}\"; do\n                if [[ \"${{COMP_WORDS[i]}}\" == \"file\" ]]; then\n                    file_idx=$i\n                    break\n                fi\n            done\n            if [[ $file_idx -ne -1 && $((COMP_CWORD - file_idx)) -eq 1 ]]; then\n                opts=\"$(dotted artifact list --raw 2>/dev/null)\"\n            else\n                opts=\"\"\n                COMPREPLY=()\n                _filedir\n                return 0\n            fi"
        );
        output = output.replace(&pattern, &replacement);
        output.push_str(
            r#"
_dotted() {
    local action_idx=-1 action="" state="" i
    for i in "${!COMP_WORDS[@]}"; do
        case "${COMP_WORDS[i]}" in
            enable|disable|show|uninstall|apply|diff|status|file|package|service)
                action_idx=$i
                action="${COMP_WORDS[i]}"
                break
                ;;
        esac
    done
    if [[ $action_idx -ge 0 ]]; then
        if [[ $action == enable ]]; then state="--state disabled"; fi
        if [[ $action == disable ]]; then state="--state enabled"; fi
        local artifact_idx=$((action_idx + 1))
        if [[ $action == file && $COMP_CWORD -gt $artifact_idx ]]; then
            local dotted_dotglob=0
            shopt -q dotglob && dotted_dotglob=1
            shopt -s dotglob
            _filedir
            [[ $dotted_dotglob -eq 1 ]] || shopt -u dotglob
            return 0
        fi
        if [[ $COMP_CWORD -eq $artifact_idx ]]; then
            COMPREPLY=( $(compgen -W "$(dotted artifact list --raw $state 2>/dev/null)" -- "${COMP_WORDS[COMP_CWORD]}") )
            return 0
        fi
    fi
    _dotted_generated "$@"
}
"#,
        );
        io::stdout().write_all(output.as_bytes())?;
    } else if shell == Shell::Zsh {
        let mut buffer = Vec::new();
        generate(shell, &mut command, "dotted", &mut buffer);
        let mut output = String::from_utf8(buffer)?;
        let targets = &[
            (
                "::artifact -- Artifact ID (repo/artifact) to enable:",
                "::artifact:__dotted_artifacts_disabled",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to disable:",
                "::artifact:__dotted_artifacts_enabled",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to remove:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to show details for:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Apply changes only from the specified artifact:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Show diff for a specific artifact only:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Filter status to a specific artifact:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to adopt into:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to adopt package into:",
                "::artifact:__dotted_artifacts",
            ),
            (
                "::artifact -- Artifact ID (repo/artifact) to adopt service into:",
                "::artifact:__dotted_artifacts",
            ),
        ];
        for (pattern, replacement) in targets {
            output = output.replace(pattern, replacement);
        }
        output = output.replace("__dotted_artifacts_default", "__dotted_artifacts_dynamic");
        for (description, helper) in [
            (
                ":artifact -- Artifact ID (repo/artifact) to show details for:_default",
                ":artifact -- Artifact ID (repo/artifact) to show details for:__dotted_artifacts_dynamic",
            ),
            (
                ":artifact -- Artifact ID (repo/artifact) to enable:_default",
                ":artifact -- Artifact ID (repo/artifact) to enable:__dotted_artifacts_dynamic",
            ),
            (
                ":artifact -- Artifact ID (repo/artifact) to disable:_default",
                ":artifact -- Artifact ID (repo/artifact) to disable:__dotted_artifacts_dynamic",
            ),
            (
                ":artifact -- Artifact ID (repo/artifact) to remove:_default",
                ":artifact -- Artifact ID (repo/artifact) to remove:__dotted_artifacts_dynamic",
            ),
            (
                ":artifact -- Artifact ID (repo/artifact) to adopt into:_default",
                ":artifact -- Artifact ID (repo/artifact) to adopt into:__dotted_artifacts_dynamic",
            ),
            (
                ":artifact -- Artifact ID (repo/artifact) to adopt package into:_default",
                ":artifact -- Artifact ID (repo/artifact) to adopt package into:__dotted_artifacts_dynamic",
            ),
        ] {
            output = output.replace(description, helper);
        }
        output = output.replace(
            "::artifact -- Artifact ID (repo/artifact) to disable:",
            "::artifact:__dotted_artifacts_enabled",
        );
        output.push_str(
            "\n\n__dotted_artifacts_dynamic() {\n    local -a artifacts\n    local state=\"\"\n    (( ${words[(I)enable]} )) && state=\"--state disabled\"\n    (( ${words[(I)disable]} )) && state=\"--state enabled\"\n    artifacts=(${(f)\"$(dotted artifact list --raw $state 2>/dev/null)\"})\n    _describe -t artifacts 'artifacts' artifacts\n}\n__dotted_artifacts_all() {\n    local -a artifacts\n    artifacts=(${(f)\"$(dotted artifact list --raw 2>/dev/null)\"})\n    _describe -t artifacts 'artifacts' artifacts\n}\n__dotted_artifacts() { __dotted_artifacts_all; }\n__dotted_artifacts_disabled() {\n    local -a artifacts\n    artifacts=(${(f)\"$(dotted artifact list --raw --state disabled 2>/dev/null)\"})\n    _describe -t artifacts 'disabled artifacts' artifacts\n}\n__dotted_artifacts_enabled() {\n    local -a artifacts\n    artifacts=(${(f)\"$(dotted artifact list --raw --state enabled 2>/dev/null)\"})\n    _describe -t artifacts 'enabled artifacts' artifacts\n}\n",
        );
        io::stdout().write_all(output.as_bytes())?;
    } else {
        generate(shell, &mut command, "dotted", &mut io::stdout());
    }
    Ok(())
}
