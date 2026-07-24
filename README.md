# Dotted

A simple, templateless, multi-[device|repo|user|distro] dotfile manager that is highly shareable and tracks system packages.

---

## Key Features

- **Decoupled Modular Artifacts**: Share modular dotfile components via independent repositories and artifacts (`[artifacts]/` or `repo/artifact`) rather than forcing a monolithic dotfiles repository.
- **Hierarchical Overrides**: Merges multi-device and multi-user configurations hierarchically (`[device]/[user].toml` -> `laptop/user.toml`) with zero templating logic needed.
- **Template-Free Variable Replacement**: Perform targeted string replacements (`[replace]` key-value mappings) directly on text dotfiles without complex template syntax.
- **System Package & Download Tracking**: Track, manifest, and declare dependencies across Arch Linux (`pacman`), Fedora (`dnf`), Ubuntu (`apt-get`), Flatpak, and web binary downloads (`curl`/`unzip`).
- **Orphan & Unclaimed Resource Audits**: Inspect system drift and discover untracked installed packages or binaries via `dotted deploy orphans`.
- **Safety First & Automatic Backups**: Existing target files modified during deployment are automatically backed up with Unix timestamps (`~/.cache/dotted/backups`). `dotted` never deletes files or runs unrequested uninstalls.
- **Deny-by-Default Git Security**: The workspace enforces a strict `.gitignore` layout to prevent accidental commits of local settings, tokens, or machine-specific secrets.
- **Rich Interactive CLI & Shell Completion**: Features dynamic terminal completion generation for Bash, Zsh, and Fish, plus interactive TUI prompts for conflict resolution and file adoption.

---

## Directory Structure

All configurations and cloned artifact repositories are stored under:
`~/.local/share/dotted/` (known as the **dotted repo**).

```text
~/.local/share/dotted/            ← Dotted Repo (private git remote recommended)
├── .gitignore
├── [dotted].toml                 ← Configures repositories & global shell env path
├── [artifacts]/                  ← Locally writable artifacts
├── [settings]/
│   ├── [device]/
│   │   ├── [user].toml           ← Applies to every device and user
│   │   └── user1.toml
│   ├── laptop/
│   │   ├── [user].toml           ← Specific device, applies to all users
│   │   └── user1.toml            ← Specific device + user pair
│   └── pc/
│       └── [user].toml
│
├── remote-repo/                  ← Git clone declared by [[repo]]
│   ├── [about].toml              ← Source of truth declaring all artifacts
│   ├── shell-config/
│   │   ├── [bin].toml            ← Optional; package dependencies & env vars
│   │   └── home/
│   │       ├── .bashrc           ← Files map relative to home/
│   │       └── .config/
│   └── system-config/
│       └── etc/
│           └── pacman.conf       ← Files map relative to root /
```

---

## Core Concepts & Architecture

### 1. Artifacts & Repositories

- **Artifact**: A self-contained bundle of dotfiles, environment variables, system packages, and download definitions.
- **Local Artifacts (`/artifact_name`)**: Stored directly in `~/.local/share/dotted/[artifacts]/<name>`.
- **Remote Artifacts (`repo_name/artifact_name`)**: Stored inside git sub-repositories under `~/.local/share/dotted/<repo_name>/<artifact_name>`.
- **`[about].toml`**: Declares available artifacts and their metadata in a repository root.

### 2. Path Mapping Rules

Files inside an artifact map directly to target filesystem paths based on top-level directory names:

- `<artifact>/home/...` $\rightarrow$ `$HOME/...` (e.g. `home/.bashrc` $\rightarrow$ `~/.bashrc`)
- `<artifact>/root/...` $\rightarrow$ `/root/...`
- `<artifact>/<etc|usr|var|...>/...` $\rightarrow$ `/<etc|usr|var|...>/...` (e.g. `etc/pacman.conf` $\rightarrow$ `/etc/pacman.conf`)

### 3. Settings & Overrides Hierarchy

Layered TOML configuration files under `[settings]/` merge settings cumulatively:

1. `[settings]/[device]/[user].toml` (Global default for all devices and users)
2. `[settings]/[device]/<current_user>.toml` (All devices for specific user)
3. `[settings]/<current_device>/[user].toml` (Specific device for all users)
4. `[settings]/<current_device>/<current_user>.toml` (Specific device and user pair)

### 4. Package & Binary Manifests (`[bin].toml`)

Artifacts can optionally include a `[bin].toml` declaring dependencies and environment variables:

```toml
[env]
EDITOR = "nvim"

[distro.archlinux]
packages = ["neovim", "starship"]

[distro.fedora]
packages = ["neovim-qt"]

[flatpak]
packages = ["com.visualstudio.code"]

[download.x86_64]
url = "https://github.com/fastfetch-cli/fastfetch/releases/download/2.30.0/fastfetch-linux-amd64.tar.gz"
zip = "fastfetch-linux-amd64.tar.gz"
path = "fastfetch"
install = "local" # "local" (~/.local/bin) or "system" (/usr/local/bin)
```

---

## Installation & Building

This project requires Rust and Cargo. It uses `just` as a command runner.

1. **Clone the repository**:

   ```bash
   git clone <repository-url>
   cd dotted
   ```

2. **Build and Run**:

   ```bash
   cargo build --release
   # Check the CLI options
   cargo run --release -- --help
   ```

3. **Development Tasks (using `just`)**:
   - List available commands: `just`
   - Run tests & linters: `just validate`

---

## CLI Reference & Usage

### 1. Workspace Commands

Manage the dotted repo environment and pull/push changes across machines.

- **Initialize a workspace**:
  ```bash
  dotted workspace init [git_url]
  ```
  _If `git_url` is provided, clones it; otherwise initializes a local repository._
- **Sync/Pull updates**:
  ```bash
  dotted workspace pull
  ```
- **Commit & Push modifications**:
  ```bash
  dotted workspace push
  ```
- **Open workspace in a new shell**:
  ```bash
  dotted workspace cd
  ```
- **Run system integrity checks**:
  ```bash
  dotted workspace doctor [config|repo|artifact|tool]
  ```

### 2. Artifact Commands

Manage the dotfile and package bundles.

- **List discovered artifacts**:
  ```bash
  dotted artifact list [filter] [--raw] [--state enabled|disabled]
  ```
- **Show detailed information & status**:
  ```bash
  dotted artifact show <artifact_id>
  ```
- **Create a new artifact structure**:
  ```bash
  dotted artifact create <artifact_name>
  ```
- **Enable/Disable artifacts**:
  ```bash
  dotted artifact enable <artifact_id>
  ```
  ```bash
  dotted artifact disable <artifact_id>
  ```
- **Uninstall artifact files & configuration**:
  ```bash
  dotted artifact uninstall <artifact_id> [-y]
  ```

### 3. Adopt Commands

Incorporate existing system files and packages into your workspace.

- **Adopt a system file**:
  ```bash
  dotted adopt file <artifact_id> [path]
  ```
  _If path is omitted, starts an interactive file browser to pick a file._
- **Adopt a system package**:
  ```bash
  dotted adopt package <artifact_id> [package_name] [--type archlinux|fedora|ubuntu|flatpak]
  ```

### 4. Deploy Commands

Inspect and apply the planned configuration changes to the system.

- **Show pending changes**:
  ```bash
  dotted deploy status [artifact_id] [--filter artifacts|files|env|packages|downloads]
  ```
- **Show line-by-line diffs**:
  ```bash
  dotted deploy diff [artifact_id]
  ```
- **Apply configuration & install dependencies**:
  ```bash
  dotted deploy apply [artifact_id] [-y]
  ```
- **Scan for unclaimed system packages or binaries**:
  ```bash
  dotted deploy orphans [--filter native|flatpak|downloads]
  ```

### 5. Repository Commands

Manage external artifact repositories configured in `[dotted].toml`.

- **List configured repositories**:
  ```bash
  dotted repo list
  ```
- **Add a repository configuration and clone it**:
  ```bash
  dotted repo add <name> <git_url>
  ```
- **Remove a repository configuration**:
  ```bash
  dotted repo remove <name>
  ```
- **Show metadata of a repository**:
  ```bash
  dotted repo about <name>
  ```

### 6. File Inventory Commands

- **List target folder contents and their tracking status**:
  ```bash
  dotted files list [--path <path>] [--depth <depth>] [--filter tracked|untracked|ignored|mixed]
  ```
  _(Status categories: `[tracked]`, `[untracked]`, `[ignored]`, or `[mixed]`)_
- **Scan target folder recursively**:
  ```bash
  dotted files scan [--path <path>] [--filter tracked|untracked|ignored|mixed]
  ```
- **Add/remove patterns to/from ignore lists**:
  ```bash
  dotted files ignore add [path]
  dotted files ignore remove [path]
  ```
  _If `path` is omitted, starts an interactive file browser to select a path._

### 7. Backup & Restore Commands

- **List available backups**:
  ```bash
  dotted backup list [timestamp] [--filter <path>]
  ```
- **Restore a backup version**:
  ```bash
  dotted backup restore <timestamp> [path]
  ```

### 8. Shell Integration

- **Generate shell completions**:
  ```bash
  dotted shell completions <bash|zsh|fish|powershell|elvish>
  ```
- **Inject environment variables into your shell**:
  ```bash
  eval "$(dotted shell env [shell])"
  ```

---

## How to Get Started (Example Walkthrough)

### Step 1: Initialize the Workspace

On a new machine, initialize your Dotted repo directory:

```bash
dotted workspace init
```

This sets up the required directory structure and configuration under `~/.local/share/dotted/`.

### Step 2: Adopt a Dotfile

Adopt your current shell configurations into a new artifact named `shell`:

```bash
dotted artifact create shell
dotted adopt file personal-dots/shell ~/.bashrc
dotted adopt file personal-dots/shell ~/.config/starship.toml
```

### Step 3: Register Package Dependencies

Ensure your favorite terminal prompt is registered to install:

```bash
dotted adopt package personal-dots/shell starship native archlinux
```

### Step 4: Review and Deploy

Check if your deployment plan has any differences or unapplied changes, then deploy them:

```bash
dotted deploy status
dotted deploy diff
dotted deploy apply
```

This will copy files to their proper places, download/install any missing packages, and prepare your system environments!
