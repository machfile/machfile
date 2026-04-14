use core::fmt;
use std::error::Error;

mod cli;
mod executer;
mod parser;

pub use cli::cli;

#[derive(Debug)]
pub struct CommandError;

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command encountered an unexpected error")
    }
}

impl Error for CommandError {}
