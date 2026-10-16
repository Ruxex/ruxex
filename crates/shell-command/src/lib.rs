//! Command parsing and safety utilities shared across Ruxex crates.

pub mod shell_detect;
pub mod shell_snapshot;
mod startup;

pub use startup::shell_startup_script;

pub mod bash;
pub(crate) mod command_safety;
pub mod parse_command;

pub use command_safety::is_dangerous_command;
