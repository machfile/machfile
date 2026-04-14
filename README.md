# Mach

Simple task runner, for when a full build system is overkill

## Installation

```shell
git clone https://github.com/machfile/mach.git
cd mach
cargo install --path .
```

### Auto-complete setup

Mach supports dynamic auto completion

#### ZSH

Add the following to your `.zshrc`

```shell
# Load mach autocompletion
_update_completion() {
    local auto_complete_output
    auto_complete_output=$(mach auto_complete zsh 2>&1)

    if [[ $? -ne 0 ]]; then
        compdef -d mach
        return
    fi

    eval "$auto_complete_output"
}
chpwd() {
    _update_completion
}
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
