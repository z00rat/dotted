use color_eyre::eyre::Result;

use crate::commands::ignore::common::{self, IgnoreKind};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    common::remove(runtime, IgnoreKind::Package, names)
}
