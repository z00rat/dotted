# Dotted workspace guide

Use this guide when editing a Dotted workspace. Use `dotted --help` and each
command's `--help` for current command syntax.

## Before changing the workspace

- Read `memory.md` when it contains preferences relevant to the task. Add a
  preference there only when the user asks you to remember it. Never store
  secrets there.
- Inspect the relevant settings, artifact manifest, and payload files before
  editing them.
- Run `dotted deploy status` and `dotted deploy diff` to inspect planned file
  changes.
- Ask before running `dotted deploy apply`. It writes files to the user's home
  or system paths.
- `dotted deploy apply` does not install packages, download binaries, or enable
  services. It reports the commands or links for those actions. Show them to
  the user; do not run them.
- `dotted artifact uninstall` can remove deployed files and paths listed in an
  artifact's `[config].remove`. Inspect those paths and get approval first.
- `dotted workspace format` formats control TOML and rewrites `AGENTS.md` from
  the installed template. Check its effects before using it in a customized
  workspace.
- `dotted workspace doctor` can repair ownership when run as root. Do not run
  it with elevated permissions unless the user requests that repair.

## Workspace model

- A local artifact ID starts with `/`, such as `/shell`.
- A repository artifact ID is `<repo>/<artifact>`, such as `community/shell`.
- An artifact's `[about].toml` entry declares its name, revision `r`, and
  description. Its `[bin].toml` manifest is optional.
- In an artifact, `home/` maps under the configured home directory, `root/`
  maps under `/root`, and any other top-level directory maps under `/`.
- `[local].toml` can set the current device name. The command-line options
  `--device` and `--user` override the detected values.

Settings load in this order:

1. `[settings]/[device]/[user].toml`
2. `[settings]/[device]/<user>.toml`
3. `[settings]/<device>/[user].toml`
4. `[settings]/<device>/<user>.toml`

The `enable`, `disable`, and `ignore` lists accumulate across these files. If
an artifact appears in both `enable` and `disable`, it stays disabled. Later
`replace` and `env` values replace earlier values with the same key.

## Artifact manifest fields

`[bin].toml` can declare environment values, packages, services, downloads,
and ignore rules. Keep system-specific packages under their distro table.

```toml
[env]
EDITOR = "nvim"

[distro.archlinux]
packages = ["neovim"]

[flatpak]
packages = ["org.example.App"]

[services.user]
units = ["syncthing.service"]

[download.x86_64]
url = "https://example.org/tool"
path = "tool"
install = "local"
```

For an archive download, use `zip` instead of `url` and set `path` to the
archive entry to extract. Set `install` to `local` or `system`. Dotted records
these downloads but does not fetch them or verify the optional `hash` field.

`[config].remove` lists paths that `artifact uninstall` removes recursively.
Treat each listed path as destructive input and review it before uninstalling.

Package and service entries describe desired state. Dotted checks installed
packages and service state, then prints suggested commands for missing items.
It does not run package managers, download commands, or `systemctl` changes.

## Useful checks

- `dotted artifact list` shows discovered artifacts.
- `dotted artifact show <id>` shows one artifact's contents and manifest.
- `dotted deploy status [id]` shows planned state.
- `dotted deploy diff [id]` shows file differences.
- `dotted deploy orphans` reports unclaimed packages, binaries, and services.
- `dotted workspace doctor` checks workspace configuration and required tools.
- For structured output, `artifact list`, `deploy status`, `deploy orphans`,
  `workspace doctor`, and `backup list` accept `--format json|toml|toon`.
  Structured results are objects with `schema_version` set to `1`.

Before finishing, report the files changed, checks run, and any suggested
package, download, or service commands for the user to review.
