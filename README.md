# Machfile

> Simple task runner, for when a full build system is overkill

This project provides both a [library](machfile/src/lib.rs) and a
[CLI utility](machfile-cli/src/lib.rs). The CLI is the primary implementation of the library, and
provides the basic functionality of the task runner. The binary is called `mach`.

## Installation

```shell
git clone https://github.com/machfile/mach.git
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

Mach can be configured with `mach.toml`, `mach.yaml` or `mach.yml` files. When inside a `git` repository, mach will
search the directory tree upwards until finding a config file (stopping at the root directory of the `git`
repository. If not inside a `git` repository, mach must be called from within the same directory as
the configuration file.

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

## Configuration

See `mach.toml`

## Development

Setting the environment variable `MACH_CONFIG_PATH` allows using a specific custom configuration
file.

```shell
MACH_CONFIG_PATH=tests/example.toml cargo run -- --help
```
