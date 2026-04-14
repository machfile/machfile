use std::process::ExitCode;

use mach::cli;

fn main() -> ExitCode {
    env_logger::init();

    match cli() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
