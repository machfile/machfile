use std::process::ExitCode;

use machfile::cli;

fn main() -> ExitCode {
    env_logger::init();

    match cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
