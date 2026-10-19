use color_eyre::eyre::{Result, eyre};
use serde::Serialize;

use crate::cli::OutputFormat;

pub(crate) fn print_structured<T: Serialize>(format: &OutputFormat, value: &T) -> Result<()> {
    let mut json_value = serde_json::to_value(value)?;
    let Some(object) = json_value.as_object_mut() else {
        return Err(eyre!(
            "structured command output must be a top-level object"
        ));
    };
    object.insert("schema_version".into(), serde_json::json!(1));
    let output = match format {
        OutputFormat::Human => return Err(eyre!("structured output requires json, toml, or toon")),
        OutputFormat::Json => serde_json::to_string_pretty(&json_value)?,
        OutputFormat::Toml => {
            let value = toml::Value::try_from(json_value)
                .map_err(|error| eyre!("convert output to TOML: {error}"))?;
            toml::to_string_pretty(&value)?
        }
        OutputFormat::Toon => toon_format::encode_default(&json_value)
            .map_err(|error| eyre!("encode TOON output: {error}"))?,
    };
    println!("{output}");
    Ok(())
}
