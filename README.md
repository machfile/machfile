![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/machfile/machfile/rust.yml?style=flat-square)
![Crates.io library version](https://img.shields.io/crates/v/machfile?style=flat-square&label=library%20version)
![Crates.io CLI version](https://img.shields.io/crates/v/machfile-cli?style=flat-square&label=CLI%20version)
![Crates.io License](https://img.shields.io/crates/l/machfile-cli?style=flat-square)


# Machfile

> Simple task runner, for when a full build system is overkill

This project provides both a [library](machfile/src/lib.rs) and a
[CLI utility](machfile-cli/src/lib.rs). The CLI is the primary implementation of the library, and
provides the basic functionality of the task runner. The binary is called `mach`.

## Installation

You can install the binary with `cargo`:

```shell
cargo install machfile-cli --locked
```

Or clone the repository and install manually:

```shell
git clone https://github.com/machfile/machfile.git
cd mach
cargo install --path .
```

### Auto-complete setup

Mach supports dynamic auto completion in ZSH and bash:

```shell
# Setup dynamic mach auto-complete
source <(mach setup_complete [zsh|bash])
```

## Configuration

Mach can be configured with `mach.toml`, `mach.yaml` or `mach.yml` files. When inside a `git` 
repository, mach will search the directory tree upwards until finding a config file (stopping at
the root directory of the `git` repository). If not inside a `git` repository, mach must be called
from within the same directory as the configuration file.

### Tasks

Machfiles are made up of several individual tasks. This is the standard unit that is normally
called. A task has a `name`, and something that can be executed. This can be a `script` or a list
of dependencies (`deps`). The task can also have an optional description (`desc`) and several
`options`.

```toml
[do_stuff]
script = "touch test"
desc = "Create a test file"
deps = ["remove_old_test"]
options.working_directory = "/tmp"
```

```yaml
do_stuff:
  script: "touch test"
  desc: "Create a test file"
  deps:
    - "remove_old_test"
  options:
    working_directory: "/tmp"
```

#### `script`

The script is a (multiline) string containing one or several commands that will be executed in
order one after the other. If provided with a multiline-string, the string is separated into the
different lines. Each command string will then be separated into the command (the first word) and
its arguments (everything after) and executed in an isolated shell. **This shell inherits the
environment from the mach process**.

Any exit code other than **0** will cancel the execution of following commands and exit the 
programm with an error.

Pipes, `cd` and other shell features do **NOT** work in the `script` tag. If you have the need for
these, write your complex script as a bash script and use that as an argument for `script`.

Either a `script` or at least **one** dependency is required for the task to be valid.

#### `desc`

Desc should contain a description for your task. The mach CLI will display this text in it's help
messages (`mach --help`) and in the autocomplete functionality.

#### `deps`

The dependencies are a list of tasks that should be executed before the task in which the
dependencies are configured. The dependencies will be executed in order, and the execution will be
stopped if any dependencies results in an error.

A task with dependencies but without a script is completely valid.

#### `options`

Options modify the task execution. **They do not affect dependencies**.

The following options are currently supported:

- `working_directory` - Sets the working directory of the `script` commands relative to the used
  configuration file
- `environment` - Dictonary of keys and values that will be injected into the execution environment
  of the `script` commands.
- `allow_args` - If true, the task can be called with additional arguments that will be passed onto
  the first command of the last executed task (the called task if it has a `script` or the last
  dependency in the execution chain).

### Environment files

Mach supports `.env` files. Values from environment files are **not** automatically passed into the
environments of tasks, instead, individual values can be used to construct environment variables for 
the called script.

Environment file reading can be deactivated by the `--no-env-file` flag and the default `.env`
filename can be overriden with the `--env-file` argument.

#### Env file value usage in scripts and `environment` values

Given a `.env` file:

```
ENVIRONMENT=prod
```

You could use `ENVIRONMENT` as a parameter for a script command:

```toml
script = "node --env=$ENVIRONMENT"
```

And you can also construct a environment variable (or pass it forward):

```toml
[env_values]
script = """printenv ENVIRONMENT
printenv BACKEND_ENVIRONMENT"""
[env_values.options.environment]
ENVIRONMENT = "$ENVIRONMENT"
BACKEND_ENVIRONMENT = "${ENVIRONMENT}_BACKEND"
```

### Configuration examples

The following is an example configuration written in `toml`:

```toml
[run]
script = "cargo run"
desc = "Run with debug logging"
options.environment.RUST_LOG = "mach=debug,info"

[clean]
script = "rm -rf target"
desc = "Remove cache and outputs"

[check_target_size]
script = "du -d1 -h"
desc = "Check directory sizes of cache dirs"
options.working_directory = "target"

[install]
script = "cargo install --path ."
deps = ["clean"]
```

The same configuration can also be written in `yaml`:

```yaml
run:
  script: "cargo run"
  desc: "Run with debug logging"
  options:
    environment:
      RUST_LOG: "mach=debug,info"

clean:
  script: "rm -rf target"
  desc: "Remove cache and outputs"

check_target_size:
  script: "du -d1 -h"
  desc: "Check directory sizes of cache dirs"
  options:
    working_directory: "target"

install:
  script: "cargo install --path ."
  deps:
    - "clean"
```

## Usage

Consult the `--help` command for usage instructions

```shell
mach --help
```

## Development

Setting the environment variable `MACH_CONFIG_PATH` allows using a specific custom configuration
file.

```shell
MACH_CONFIG_PATH=tests/example.toml cargo run -- --help
```
