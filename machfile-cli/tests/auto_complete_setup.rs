use assert_cmd::cargo::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn auto_complete_setup_requires_shell() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("setup_complete");
    cmd.assert().failure().stderr(predicate::str::contains(
        "error: the following required arguments were not provided:\n  <shell>",
    ));

    Ok(())
}

#[test]
fn auto_complete_setup_provides_script() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("setup_complete").arg("zsh");
    cmd.assert().success().stdout(predicate::str::contains(
        "local auto_complete_output\n    auto_complete_output=$(mach auto_complete zsh 2>&1)",
    ));

    Ok(())
}

#[test]
fn auto_complete_contains_command() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str("[test]\nscript = \"touch asdf.txt\"\ndesc = \"Create test file\"")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("auto_complete").arg("zsh").current_dir(&temp);
    cmd.assert().success().stdout(predicate::str::contains(
        r#"
(( $+functions[_mach_commands] )) ||
_mach_commands() {
    local commands; commands=(
'auto_complete:' \
'setup_complete:' \
'test:Create test file' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'mach commands' commands "$@"
}"#,
    ));

    temp.close().unwrap();
    Ok(())
}
