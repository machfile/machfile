//! # `machfile-cli`
//! > Not a build system
//!
//! This is the `mach` CLI utility.
//!
//! ## Configuration
//!
//! The following is an example configuration written in `toml`:
//!
//! ```toml
//! [run]
//! script = "cargo run"
//! desc = "Run with debug logging"
//! options.environment.RUST_LOG = "mach=debug,info"
//!
//! [clean]
//! script = "rm -rf target"
//! desc = "Remove cache and outputs"
//!
//! [check_target_size]
//! script = "du -d1 -h"
//! desc = "Check directory sizes of cache dirs"
//! options.working_directory = "target"
//!
//! [install]
//! script = "cargo install --path ."
//! deps = ["clean"]
//! ```
//! The same configuration can be written in `yaml` as follows:
//!
//! ```yaml
//! run:
//!   script: "cargo run"
//!   desc: "Run with debug logging"
//!   options:
//!     environment:
//!       RUST_LOG: "mach=debug,info"
//!
//! clean:
//!   script: "rm -rf target"
//!   desc: "Remove cache and outputs"
//!
//! check_target_size:
//!   script: "du -d1 -h"
//!   desc: "Check directory sizes of cache dirs"
//!   options:
//!     working_directory: "target"
//!
//! install:
//!   script: "cargo install --path ."
//!   deps:
//!     - "clean"
//! ```
//!
//! ## Usage
//!
//! Run the command without arguments to show all available tasks:
//!
//! ```shell
//! mach
//! ```
//!
//! Run a given task by providing its name to the command:
//!
//! ```shell
//! mach build
//! ```
//!
//! ### Shell auto completion
//!
//! `mach` supports completions for `bash` and `zsh`. For this to work, source the setup command
//! inside your shell initialization script (`.zshrc` or `.bashrc`, for example) providing the name
//! of your shell.
//!
//! ```shell
//! source <(mach setup_complete zsh)
//! ```
//!
//! <div class="warning">Completion is only available when the binary was build with the `complete`
//! feature. This feature is part of the default features.</div>

mod cli;
mod config_info;

#[cfg(feature = "complete")]
mod complete;

pub use cli::cli;
