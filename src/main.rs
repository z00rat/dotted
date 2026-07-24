fn main() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .display_location_section(false)
        .panic_section("Consider filing a bug report at https://github.com/z00rat/dotted")
        .install()?;
    if let Err(error) = dotted::run() {
        eprintln!("error: {error:?}");
        std::process::exit(1);
    }
    Ok(())
}
