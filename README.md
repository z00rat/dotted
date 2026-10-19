# Dotted

A simple, templateless, multi-[device|repo|user|distro] dotfile manager that is highly shareable and tracks system packages & services.

Dotted uses plain files and TOML configuration, with no templates. It also tracks packages, downloads, and systemd services. Dotted prints commands for those dependencies; you choose whether to run them.

## Install

Dotted requires Rust and Cargo.

```sh
git clone <repository-url>
cd dotted
cargo install --path .
```

Or build and run from the checkout:

```sh
cargo run -- --help
```

## Get started

Initialize a workspace, then create an artifact and adopt files into it:

```sh
dotted workspace init
dotted artifact create shell
dotted adopt file /shell ~/.bashrc ~/.config/starship.toml
```

Review the file changes before applying them:

```sh
dotted deploy status
dotted deploy diff
dotted deploy apply
```

`deploy apply` copies dotfiles and prints suggested commands for packages, downloads, and services. It does not install those dependencies or enable services.

## How it works

A workspace lives at `~/.local/share/dotted` by default. It contains workspace settings and local artifacts. It can also contain cloned repositories of shared artifacts.

An artifact groups files and optional dependency declarations. Top-level folders map to system paths:

| Artifact path | Target |
| --- | --- |
| `home/.bashrc` | `~/.bashrc` |
| `root/.config/tool/config` | `/root/.config/tool/config` |
| `etc/example.conf` | `/etc/example.conf` |

The workspace applies settings in this order, from general to specific:

1. `settings/[device]/[user].toml`
2. `settings/[device]/<user>.toml`
3. `settings/<device>/[user].toml`
4. `settings/<device>/<user>.toml`

Later settings override earlier ones. Dotted detects the device, user, and supported distribution, or accepts overrides with `--device`, `--user`, and `--distro`.

## Workspace layout

```text
~/.local/share/dotted/
├── [dotted].toml
├── settings/
│   ├── [device]/
│   │   ├── [user].toml
│   │   └── <user>.toml
│   └── <device>/
│       ├── [user].toml
│       └── <user>.toml
├── artifacts/
│   └── <local-artifact>/
└── <repo>/
    ├── [about].toml
    └── <artifact>/
        ├── [bin].toml
        └── home/
```

`[device]` and `[user]` are literal fallback names. Replace `<device>`, `<user>`, `<repo>`, and `<local-artifact>` with actual names. Repository metadata in `[about].toml` declares its artifacts. Each artifact may contain a `[bin].toml` manifest for environment variables and dependencies. The workspace config is `[dotted].toml`.

For example:

```toml
[env]
EDITOR = "nvim"

[distro.archlinux]
packages = ["neovim", "starship"]

[flatpak]
packages = ["org.example.App"]

[services.user]
units = ["syncthing.service"]
```

Package managers support Arch Linux, Fedora, Ubuntu, and Flatpak. Dotted can also track downloadable binaries and user or system systemd units.

## Commands

```text
workspace init [git_url]     Create or clone a workspace
workspace pull               Pull workspace repositories
workspace push               Commit and push workspace changes
workspace doctor [category]  Check workspace configuration
workspace format             Format workspace TOML files

repo add|remove|list|about   Manage artifact repositories
artifact list|show|create    Find and manage artifacts
artifact enable|disable      Select artifacts for deployment
adopt file|package|service   Add existing files or dependencies to an artifact

deploy status                Preview pending changes (also: deploy plan)
deploy diff                  Compare files with the system
deploy apply                 Copy files and print dependency commands
deploy orphans               Find untracked packages, downloads, or services

ignore file|package|service  Manage ignored system resources
backup list                  List saved file backups
backup restore               Restore a backup
shell env                    Print shell environment setup
shell completions            Generate shell completions
```

Run `dotted --help` or `dotted <command> --help` for arguments and options. Commands that list data support `--format human|json|toml|toon` where documented by `--help`; human output is the default. Structured output includes `schema_version`.

## Development

Use `just` to list project tasks. `just check` runs formatting and lint checks. `just validate` runs the full validation task.
