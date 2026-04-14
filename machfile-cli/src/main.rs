use std::process::ExitCode;

use log::debug;

use machfile_cli::cli;

fn main() -> ExitCode {
    env_logger::init();
    debug!("Running with debug");

    match cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
