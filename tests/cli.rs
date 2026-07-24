#![allow(clippy::unwrap_used, clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Fixture {
    _temp: TempDir,
    meta: PathBuf,
    home: PathBuf,
    root: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir");
        let meta = temp.path().join("meta");
        let home = temp.path().join("home");
        let root = temp.path().join("root");
        let bin = temp.path().join("bin");
        copy_dir(Path::new("tests/fixtures"), &meta);
        fs::create_dir_all(&home).expect("home dir");
        fs::create_dir_all(&root).expect("root dir");
        fs::create_dir_all(&bin).expect("bin dir");
        write_fake_tools(&bin);
        Self {
            _temp: temp,
            meta,
            home,
            root,
            bin,
        }
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("dotted").expect("binary");
        cmd.args([
            "--dotted-dir",
            self.meta.to_str().expect("meta utf8"),
            "--home-dir",
            self.home.to_str().expect("home utf8"),
            "--root-dir",
            self.root.to_str().expect("root utf8"),
            "--device",
            "laptop",
            "--user",
            "user1",
            "--no-color",
        ]);
        cmd.env_remove("SUDO_USER");
        cmd.env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()));
        cmd
    }
}

fn write_fake_tools(bin: &Path) {
    for tool in ["pacman", "dnf", "apt-get", "flatpak"] {
        write_executable(bin.join(tool), "#!/usr/bin/env sh\nexit 0\n");
    }
    write_executable(bin.join("sudo"), "#!/usr/bin/env sh\nexec \"$@\"\n");
    write_executable(
        bin.join("curl"),
        "#!/usr/bin/env sh\nout=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"--output\" ]; then shift; out=\"$1\"; fi\n  shift || true\ndone\n[ -n \"$out\" ] && printf downloaded > \"$out\"\n",
    );
    write_executable(bin.join("unzip"), "#!/usr/bin/env sh\nprintf extracted\n");
}

fn write_executable(path: PathBuf, content: &str) {
    fs::write(&path, content).expect("write fake tool");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o755);
    }
    fs::set_permissions(path, permissions).expect("chmod fake tool");
}

fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create destination");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.expect("dir entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).expect("copy file");
        }
    }
}

#[test]
fn deploy_status_displays_example_settings_and_artifacts() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("/git"))
        .stdout(predicate::str::contains("repo2/neovim"))
        .stdout(predicate::str::contains("repo2/sysconfig"))
        .stdout(predicate::str::contains("/etc/config.conf"));
}

#[test]
fn artifact_list_ignores_unconfigured_top_level_repository() {
    let fixture = Fixture::new();
    let stray = fixture.meta.join("stray");
    fs::create_dir_all(stray.join("ghost")).expect("stray repo");
    fs::write(
        stray.join("[about].toml"),
        "[about.ghost]\nr = 1\ndescription = \"ignored\"\n",
    )
    .expect("about file");
    fs::write(stray.join("ghost/[bin].toml"), "").expect("bin file");

    fixture
        .cmd()
        .args(["artifact", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("stray/ghost").not());
}

#[test]
fn shell_completions_includes_state_and_path_candidates() {
    let bash = Command::cargo_bin("dotted")
        .expect("binary")
        .args(["shell", "completions", "bash"])
        .output()
        .expect("bash completion");
    let bash = String::from_utf8_lossy(&bash.stdout);
    assert!(bash.contains("--state disabled"));
    assert!(bash.contains("_filedir"));

    let fish = Command::cargo_bin("dotted")
        .expect("binary")
        .args(["shell", "completions", "fish"])
        .output()
        .expect("fish completion");
    let fish = String::from_utf8_lossy(&fish.stdout);
    assert!(fish.contains("__fish_complete_path"));
}

#[test]
fn deploy_apply_writes_files_and_expands_replacements() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("download:"));

    let git_config =
        fs::read_to_string(fixture.home.join(".config/git/config")).expect("git config");
    assert!(git_config.contains("email = user1@example.com"));
    assert!(git_config.contains("name = user1"));
    assert!(fixture.root.join("etc/config.conf").exists());
    assert!(fixture.home.join(".config/nvim/init.lua").exists());
    assert!(fixture.home.join(".config/dotted/env.sh").exists());
}

#[test]
fn deploy_status_fails_on_duplicate_target_paths() {
    let fixture = Fixture::new();
    let duplicate = fixture.meta.join("repo2/neovim/home/.config/git/config");
    fs::create_dir_all(duplicate.parent().expect("parent")).expect("duplicate parent");
    fs::write(duplicate, "[user]\nname = duplicate\n").expect("duplicate file");

    fixture
        .cmd()
        .args(["deploy", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate target path"));
}

#[test]
fn artifact_disable_updates_settings_to_hide_artifact() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["artifact", "disable", "repo2/neovim"])
        .assert()
        .success();
    fixture
        .cmd()
        .args(["deploy", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo2/neovim").not());
}

#[test]
fn artifact_uninstall_deletes_files_and_disables_artifact() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();
    fixture
        .cmd()
        .args(["artifact", "uninstall", "/git", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("disabled /git"));

    assert!(!fixture.home.join(".config/git/config").exists());
    assert!(fixture.home.join(".cache/dotted/backups").exists());
}

#[test]
fn workspace_doctor_validates_healthy_fixture() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["workspace", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Doctor: ok"));
}

#[test]
fn shell_env_outputs_export_vars_and_logs_overrides() {
    let fixture = Fixture::new();
    let bin = fixture.meta.join("[artifacts]/git/[bin].toml");
    let mut content = fs::read_to_string(&bin).expect("bin toml");
    content.push_str("\"EDITOR\" = \"vim\"\n");
    fs::write(bin, content).expect("write override");

    fixture
        .cmd()
        .args(["shell", "env"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export EDITOR"))
        .stderr(predicate::str::contains("env override:"));
}

#[test]
fn artifact_disable_prevents_file_deployment() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();
    assert!(fixture.root.join("etc/config.conf").exists());

    fs::remove_file(fixture.root.join("etc/config.conf")).unwrap();

    fixture
        .cmd()
        .args(["artifact", "disable", "repo2/sysconfig"])
        .assert()
        .success();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();
    assert!(!fixture.root.join("etc/config.conf").exists());
}

#[test]
fn adopt_file_and_package_updates_artifact_manifest() {
    let fixture = Fixture::new();
    let file_to_adopt = fixture.home.join(".bashrc");
    fs::write(&file_to_adopt, "# bashrc content").expect("write bashrc");

    fixture
        .cmd()
        .args(["adopt", "file", "/git", file_to_adopt.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("adopted"));

    assert!(fixture.meta.join("[artifacts]/git/home/.bashrc").exists());

    fixture
        .cmd()
        .args([
            "adopt",
            "package",
            "/git",
            "starship",
            "--type",
            "archlinux",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "added native (archlinux) package starship",
        ));

    let bin_toml = fs::read_to_string(fixture.meta.join("[artifacts]/git/[bin].toml")).unwrap();
    assert!(bin_toml.contains("starship"));
}

#[test]
fn files_list_and_ignore_add_remove() {
    let fixture = Fixture::new();
    let test_file = fixture.home.join("test_file.txt");
    fs::write(&test_file, "content").unwrap();

    fixture
        .cmd()
        .args(["files", "list", "--path", fixture.home.to_str().unwrap()])
        .assert()
        .success();

    fixture
        .cmd()
        .args(["files", "ignore", "add", test_file.to_str().unwrap()])
        .assert()
        .success();

    fixture
        .cmd()
        .args(["files", "ignore", "remove", test_file.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn repo_list_about_and_remove_manage_configuration() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["repo", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo2"));

    fixture
        .cmd()
        .args(["repo", "about", "repo2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Neovim config"));

    fixture
        .cmd()
        .args(["repo", "remove", "repo2"])
        .assert()
        .success();

    let dotted_toml = fs::read_to_string(fixture.meta.join("[dotted].toml")).unwrap();
    assert!(!dotted_toml.contains("repo2"));
}

#[test]
fn backup_list_displays_created_backups() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();

    // Modify a deployed file so apply backs it up
    let git_config_path = fixture.home.join(".config/git/config");
    fs::write(&git_config_path, "[user]\n  name = modified\n").unwrap();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();

    fixture.cmd().args(["backup", "list"]).assert().success();
}

#[test]
fn artifact_create_scaffolds_directory_structure() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["artifact", "create", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("created /zsh"));

    assert!(fixture.meta.join("[artifacts]/zsh").exists());
}

#[test]
fn deploy_diff_outputs_unified_line_differences() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();

    let git_config = fixture.home.join(".config/git/config");
    fs::write(&git_config, "[user]\nemail = changed@example.com\n").unwrap();

    fixture
        .cmd()
        .args(["deploy", "diff"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-email = changed@example.com"))
        .stdout(predicate::str::contains("+    email = user1@example.com"));
}

#[test]
fn deploy_orphans_audits_unclaimed_packages() {
    let fixture = Fixture::new();

    fixture.cmd().args(["deploy", "orphans"]).assert().success();
}

#[test]
fn files_scan_recursively_lists_paths() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["files", "scan", "--path", fixture.home.to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn workspace_init_and_cd_manage_local_directory() {
    let temp = tempfile::TempDir::new().unwrap();
    let dotted_dir = temp.path().join("dotted_workspace");

    let mut cmd = assert_cmd::Command::cargo_bin("dotted").unwrap();
    cmd.args([
        "--dotted-dir",
        dotted_dir.to_str().unwrap(),
        "workspace",
        "init",
    ]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("initialized ."));

    assert!(dotted_dir.join("[dotted].toml").exists());
    assert!(dotted_dir.join("[artifacts]").exists());

    let fixture = Fixture::new();
    fixture.cmd().args(["workspace", "cd"]).assert().success();
}

#[test]
fn backup_restore_reverts_file_to_previous_snapshot() {
    let fixture = Fixture::new();

    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();

    let git_config_path = fixture.home.join(".config/git/config");
    let original = fs::read_to_string(&git_config_path).unwrap();

    fs::write(&git_config_path, "[user]\nname = modified\n").unwrap();
    fixture
        .cmd()
        .args(["deploy", "apply", "--yes"])
        .assert()
        .success();

    let backups_dir = fixture.home.join(".cache/dotted/backups");
    let mut entries: Vec<_> = fs::read_dir(&backups_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if let Some(latest) = entries.last() {
        let ts = latest.file_name().into_string().unwrap();
        fixture
            .cmd()
            .args(["backup", "restore", &ts])
            .assert()
            .success();

        let restored = fs::read_to_string(&git_config_path).unwrap();
        assert_eq!(restored, original);
    }
}
