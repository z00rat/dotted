use crate::types::{PlannedFile, Runtime};
use color_eyre::eyre::Result;
use similar::{ChangeTag, TextDiff};
use std::io::{self, Write};

pub(crate) fn style(text: &str, color_code: &str, runtime: &Runtime) -> String {
    if runtime.no_color {
        text.to_string()
    } else {
        format!("\x1b[{color_code}m{text}\x1b[0m")
    }
}

#[allow(clippy::match_same_arms)]
pub(crate) fn color_to_ansi(name: &str) -> &'static str {
    match name {
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "bright-black" => "90",
        "bright-red" => "91",
        "bright-green" => "92",
        "bright-yellow" => "93",
        "bright-blue" => "94",
        "bright-magenta" => "95",
        "bright-cyan" => "96",
        "bright-white" => "97",
        _ => "37",
    }
}

pub(crate) fn is_terminal_color(value: &str) -> bool {
    matches!(
        value,
        "black"
            | "red"
            | "green"
            | "yellow"
            | "blue"
            | "magenta"
            | "cyan"
            | "white"
            | "bright-black"
            | "bright-red"
            | "bright-green"
            | "bright-yellow"
            | "bright-blue"
            | "bright-magenta"
            | "bright-cyan"
            | "bright-white"
    )
}

pub(crate) fn confirm(prompt: &str, no_color: bool) -> Result<bool> {
    if no_color {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let answer = line.trim().to_ascii_lowercase();
        Ok(answer.is_empty() || answer == "y" || answer == "yes")
    } else {
        cliclack::confirm(prompt)
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))
    }
}

pub(crate) fn print_line_diff(
    left_title: &str,
    right_title: &str,
    left_text: &str,
    right_text: &str,
    runtime: &Runtime,
) {
    let diff = TextDiff::from_lines(left_text, right_text);
    println!(
        "--- {}\n+++ {}",
        style(left_title, "31;1", runtime),
        style(right_title, "32;1", runtime)
    );
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                print!("-{}", style(change.value(), "31", runtime));
            }
            ChangeTag::Insert => {
                print!("+{}", style(change.value(), "32", runtime));
            }
            ChangeTag::Equal => {
                print!(" {}", change.value());
            }
        }
    }
}

pub(crate) fn print_new_file_content(title: &str, text: &str, runtime: &Runtime) {
    let col_width = 78;
    let hdr = format!(" [NEW FILE] {title:<col_width$} ");
    println!("{}", style(&hdr, "32;1", runtime));
    let border = "━".repeat(col_width);
    println!("{}", style(&border, "36", runtime));
    for line in text.lines() {
        println!("+ {}", style(line, "32", runtime));
    }
}

pub(crate) fn show_file_diff(file: &PlannedFile, current: &[u8], runtime: &Runtime) {
    if let (Ok(old), Some(new)) = (String::from_utf8(current.to_vec()), &file.text) {
        print_line_diff("Current (on-disk)", "New (planned)", &old, new, runtime);
    } else {
        println!(
            "{}",
            style("[binary or non-UTF-8 content differs]", "33", runtime)
        );
    }
}
