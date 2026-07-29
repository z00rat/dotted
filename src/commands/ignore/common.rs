use color_eyre::eyre::{Result, bail};

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

#[derive(Clone, Copy)]
pub(crate) enum IgnoreKind {
    Package,
    Service,
}

impl IgnoreKind {
    fn entries(self, file: &mut SettingsFile) -> &mut Vec<String> {
        match self {
            Self::Package => &mut file.ignore.package,
            Self::Service => &mut file.ignore.service,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Service => "service",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Package => "Packages",
            Self::Service => "Services",
        }
    }
}

fn read_settings(runtime: &Runtime) -> Result<(std::path::PathBuf, SettingsFile)> {
    let path = settings_path(runtime);
    let file = if path.exists() {
        crate::types::read_toml(&path)?
    } else {
        SettingsFile::default()
    };
    Ok((path, file))
}

fn prompt_one(kind: IgnoreKind) -> Result<Option<String>> {
    print!("Enter {} name to ignore: ", kind.noun());
    std::io::Write::flush(&mut std::io::stdout())?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let value = input.trim().to_owned();
    Ok((!value.is_empty()).then_some(value))
}

pub(crate) fn add(runtime: &Runtime, kind: IgnoreKind, values: &[String]) -> Result<()> {
    let targets = if values.is_empty() {
        let Some(value) = prompt_one(kind)? else {
            println!("No {} specified.", kind.noun());
            return Ok(());
        };
        vec![value]
    } else {
        values.to_vec()
    };
    let (path, mut file) = read_settings(runtime)?;
    let entries = kind.entries(&mut file);
    for value in targets {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if entries.iter().any(|entry| entry == value) {
            println!(
                "{} '{}' is already in ignore list.",
                kind.title().trim_end_matches('s'),
                style(value, "33", runtime)
            );
        } else {
            entries.push(value.to_owned());
            println!("Ignored {} {}", kind.noun(), style(value, "32", runtime));
        }
    }
    entries.sort();
    crate::types::write_toml(&path, &file)
}

fn select_entry(runtime: &Runtime, kind: IgnoreKind, entries: &[String]) -> Result<String> {
    if runtime.no_color {
        println!("Select a {} ignore entry to remove:", kind.noun());
        for (index, entry) in entries.iter().enumerate() {
            println!("  [{index}] {entry}");
        }
        loop {
            print!("Enter number to remove: ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            if let Some(index) = line
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|index| *index < entries.len())
            {
                return Ok(entries[index].clone());
            }
            println!("Invalid selection.");
        }
    }

    let mut select = cliclack::select(format!("Select {} ignore entry to remove:", kind.noun()));
    for (index, entry) in entries.iter().enumerate() {
        select = select.item(index, entry, "");
    }
    Ok(entries[select.interact()?].clone())
}

pub(crate) fn remove(runtime: &Runtime, kind: IgnoreKind, values: &[String]) -> Result<()> {
    let path = settings_path(runtime);
    if !path.exists() {
        println!("No ignore configuration exists.");
        return Ok(());
    }
    let mut file: SettingsFile = crate::types::read_toml(&path)?;
    if kind.entries(&mut file).is_empty() {
        println!("Ignored {} list is empty.", kind.noun());
        return Ok(());
    }

    if values.is_empty() {
        let selection = {
            let entries = kind.entries(&mut file);
            select_entry(runtime, kind, entries)?
        };
        kind.entries(&mut file).retain(|entry| entry != &selection);
        crate::types::write_toml(&path, &file)?;
        println!(
            "Removed {} ignore entry {}",
            kind.noun(),
            style(&selection, "33", runtime)
        );
        return Ok(());
    }

    let entries = kind.entries(&mut file);
    for value in values {
        let value = value.trim();
        if entries.iter().any(|entry| entry == value) {
            entries.retain(|entry| entry != value);
            println!(
                "Removed {} ignore entry {}",
                kind.noun(),
                style(value, "33", runtime)
            );
        } else {
            bail!(
                "{} ignore entry '{value}' not found in settings.",
                kind.noun().to_ascii_uppercase()
            );
        }
    }
    crate::types::write_toml(&path, &file)
}

pub(crate) fn list(runtime: &Runtime, title: &str, entries: &std::collections::BTreeSet<String>) {
    println!("{}", style(title, "36;1", runtime));
    if entries.is_empty() {
        println!("  (none)");
    } else {
        for entry in entries {
            println!("  - {}", style(entry, "90", runtime));
        }
    }
}
