# Dotted AI Assistant Guide

This document contains procedural rules for AI agents that manage a **Dotted** workspace.

---

## 1. Safety Rules

1. **NO UNSANCTIONED SYSTEM MODIFICATIONS**:
   - The AI MUST NOT execute commands that modify the target system state (e.g. `dotted deploy apply`).
   - The AI MUST request user execution for system changes.
   - The AI MAY run read-only commands (`dotted deploy status`, `dotted deploy diff`, `dotted workspace doctor`).

2. **FILE DELETION SAFETY**:
   - The AI MUST NOT run `rm` or `rm -rf`.
   - The AI MUST use `trash` to remove files or directories.

---

## 2. Directory Layout

All workspace files live in the Dotted directory: `~/.local/share/dotted/`.

```text
~/.local/share/dotted/
├── AGENTS.md                     ← Global agent instructions and rules
├── memory.md                     ← Machine and user memory state
├── [dotted].toml                 ← Global control file
├── [local].toml                  ← Machine override file (Git ignores this file)
├── [artifacts]/                  ← Local artifacts directory
│   └── <artifact>/
│       ├── [bin].toml            ← Artifact manifest file
│       └── home/                 ← Payload files relative to $HOME
├── [settings]/                   ← Layered settings overrides
│   ├── [device]/
│   │   ├── [user].toml           ← Layer 1: All devices, all users
│   │   └── <user>.toml           ← Layer 2: All devices, specific user
│   └── <device>/
│       ├── [user].toml           ← Layer 3: Specific device, all users
│       └── <user>.toml           ← Layer 4: Specific device and user
└── <repo>/                       ← Remote Git repository clone
    ├── [about].toml              ← Repository manifest
    └── <artifact>/
        ├── [bin].toml            ← Artifact manifest file
        ├── home/                 ← Maps to $HOME
        └── root/                 ← Maps to /
```

---

## 3. File Types & Schemas

### 3.1 `[dotted].toml` (Global Control File)

- **Location**: `~/.local/share/dotted/[dotted].toml`

```toml
[config]
v = "0.1.0"
env_path = ["~/.config/dotted/env.sh"]
archlinux = "sudo pacman -S --needed --noconfirm"
fedora = "sudo dnf install -y"
ubuntu = "sudo apt-get install -y"

[[repo]]
name = "community"
url = "https://github.com/example/dotted-community.git"
branch = "main"
tag = "v1.0.0"
revision = "a1b2c3d"

[color]
success = "green"
warning = "yellow"
error = "red"
info = "cyan"
muted = "bright-black"
installed = "blue"
diff = "yellow"
tracked = "bright-green"
partial = "bright-cyan"
untracked = "bright-yellow"
ignored = "bright-black"
masked = "bright-magenta"
```

### 3.2 `[local].toml` (Local Device Override File)

- **Location**: `~/.local/share/dotted/[local].toml`

```toml
device = "laptop-nitro"
```

### 3.3 `[about].toml` (Repository Manifest)

- **Location**: `<repo>/[about].toml` or `~/.local/share/dotted/[about].toml`

```toml
[about.shell]
r = 1
description = "Shell configuration"

[maintainer]
name = "User"
email = "user@example.com"
```

### 3.4 Settings Files (`[settings]/**/*.toml`)

- **Location**: `~/.local/share/dotted/[settings]/<device>/<user>.toml`
- **Merging Hierarchy** (Later layer overrides earlier layer):
  1. `[settings]/[device]/[user].toml`
  2. `[settings]/[device]/<user>.toml`
  3. `[settings]/<device>/[user].toml`
  4. `[settings]/<device>/<user>.toml`

```toml
[artifacts]
enable = ["/shell", "community/desktop"]
disable = ["/legacy"]

[ignore]
folder = ["~/.cache", "~/download", "/tmp/"]
file = ["~/.bash_history"]
package = ["vim"]
service = ["bluetooth.service"]

[replace]
"__FONT_SIZE__" = "12"

[env]
EDITOR = "nvim"
```

### 3.5 `[bin].toml` (Artifact Manifest)

- **Location**: `<artifact>/[bin].toml`

```toml
[env]
STARSHIP_CONFIG = "~/.config/starship.toml"

[distro.archlinux]
packages = ["starship", "zsh"]

[distro.fedora]
packages = ["starship"]

[flatpak]
packages = ["com.visualstudio.code"]

[services.user]
units = ["syncthing.service"]

[services.system]
units = ["docker.service"]

[download.x86_64]
url = "https://example.com/tool.tar.gz"
zip = "tool.tar.gz"
path = "tool"
hash = "sha256:..."
install = "local" # Allowed: "local" (~/.local/bin) or "system" (/usr/local/bin)

[ignore]
folder = ["~/.cache"]
file = ["temp.log"]
package = ["bash"]
service = ["systemd.service"]

[config]
remove = ["old-binary"]
```

### 3.6 Target Payload Mapping

- `<artifact>/home/...` $\rightarrow$ `$HOME/...`
- `<artifact>/root/...` $\rightarrow$ `/root/...`
- `<artifact>/<dir>/...` $\rightarrow$ `/<dir>/...`

### 3.7 `.gitignore` File

- **Location**: `~/.local/share/dotted/.gitignore`

```gitignore
*
![dotted].toml
![settings]/
![settings]/**
![artifacts]/
![artifacts]/**
!AGENTS.md
!memory.md
!.gitignore
```

---

## 4. Artifact Identifier Syntax

- **Local Artifact**: Starts with `/` (e.g. `/shell`).
- **Remote Artifact**: Uses `<repo>/<artifact>` (e.g. `community/zsh`).

---

## 5. Arch Linux Discovery Commands

Execute these commands to audit system state and report untracked items to the user:

1. **Find Unmanaged `/etc` Files**:

   ```bash
   sudo lostfiles | grep "/etc"
   ```

2. **List Explicit Packages**:

   ```bash
   pacman -Qeq | expac -Q "%-30n %d" -
   ```

3. **List Modified `/etc` Package Files**:

   ```bash
   pacman -Qii | awk '/^Backup Files/ {in_bf=1} /^[A-Z][a-zA-Z0-9 ]*:/ && !/^Backup Files/ {in_bf=0} in_bf && /\/etc\// && /\[modified\]/ {for(i=1;i<=NF;i++) if($i ~ /^\/etc\//) print $i}' | sort
   ```

---

## 6. Memory Protocol (`memory.md`)

1. Read `memory.md` at the start of a session.
2. Write user-specific preferences to `memory.md`.

---

## 7. Execution Checklist for AI

1. Read `memory.md`.
2. Use `dotted deploy status` to check changes.
3. Do NOT run `dotted deploy apply`. Request user execution.
4. Use `trash` for file deletions. Do NOT use `rm`.
5. Place user dotfiles in `<artifact>/home/`.
6. **Settings Modifications**: Add global overrides to `[settings]/[device]/[user].toml`. Put machine-specific changes in `[settings]/<device>/<user>.toml`.
