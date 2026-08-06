use super::formatting::color_to_ansi;
use crate::types::Runtime;

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
