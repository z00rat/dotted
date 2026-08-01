/// Status Color Helper
///
/// Resolves user-configured terminal ANSI color codes for file/directory tracking status tags
/// (`[tracked]`, `[partial]`, `[untracked]`, `[ignored]`, `[masked]`) based on `[dotted].toml` settings.
use crate::types::Runtime;
use crate::utils::color_to_ansi;

pub(crate) fn status_color(status: &str, runtime: &Runtime) -> String {
    let color_name = match status {
        "tracked" => &runtime.dotted.color.tracked,
        "partial" => &runtime.dotted.color.partial,
        "untracked" => &runtime.dotted.color.untracked,
        "ignored" => &runtime.dotted.color.ignored,
        "masked" => &runtime.dotted.color.masked,
        _ => "white",
    };
    color_to_ansi(color_name).to_string()
}
