pub(crate) mod formatting;
pub(crate) mod fs;
pub(crate) mod ignore;
pub(crate) mod plan;
pub(crate) mod shell;
pub(crate) mod status_color;
pub(crate) mod sys;

pub(crate) use formatting::{
    confirm, is_terminal_color, print_banner, print_new_file_content, show_file_diff, style,
};
pub(crate) use fs::{
    backup_file, cmp_dir_entries, cmp_walkdir_entries, preserve_source_permissions,
};
pub(crate) use shell::run_git;
pub(crate) use status_color::status_color;
pub(crate) use sys::{
    command_exists, command_lines, is_flatpak_installed, is_package_installed, is_service_active,
    is_service_enabled, native_package_command, shell_join,
};
