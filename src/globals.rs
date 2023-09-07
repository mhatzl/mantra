//! Contains global parameter and static settings that are available to all commands.

use std::path::PathBuf;

use clap::Args;

/// Parameters needed for all commands.
#[derive(Args, Debug, Clone)]
pub struct GlobalParameter {
    /// The folder that is searched recursively for defined requirements.
    ///
    /// [req:wiki]
    #[arg(index = 1, required = true)]
    pub req_folder: PathBuf,

    /// The folder that is searched recursively for requirement references.
    /// If not set, the current folder is used.
    #[arg(index = 2, required = false, default_value = "./")]
    pub proj_folder: PathBuf,
}

/// Flag to indicate that functions should exit on first error.
/// Setting this to `false` is useful to collect multiple errors before exiting.
static EARLY_EXIT_ON_ERROR: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(true);

/// Function returning `true` if errors should not lead to early exits.
/// This is useful to capture more than one error during command execution.
pub fn early_exit() -> bool {
    EARLY_EXIT_ON_ERROR.load(std::sync::atomic::Ordering::Relaxed)
}

/// Disables early exit on errors.
///
/// [req:check]
pub fn disable_early_exit() {
    EARLY_EXIT_ON_ERROR.store(false, std::sync::atomic::Ordering::Relaxed);
}
