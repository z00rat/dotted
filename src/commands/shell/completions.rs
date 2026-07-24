/// CLI Command: `shell completions <shell>`
///
/// What it does:
/// Generates and prints the shell autocompletion script for the dotted CLI to stdout.
///
/// Variations:
/// None (requires target shell argument, e.g. `bash`, `zsh`, `fish`, `powershell`, `elvish`).
///
/// Decisions & Logic Branches:
/// - Invokes clap's completion generator (`crate::cli::completions`) for the specified shell to output autocompletion code.
use color_eyre::eyre::Result;

pub fn run(shell: clap_complete::Shell) -> Result<()> {
    crate::cli::completions(shell)
}
