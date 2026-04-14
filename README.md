# Mach

Simple task runner, for when a full build system is overkill

## Installation

```shell
git clone https://github.com/machfile/mach.git
cd mach
cargo install --path .
```

## Usage

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
