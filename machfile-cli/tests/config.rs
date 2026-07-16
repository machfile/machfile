use assert_cmd::cargo::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn config_from_toml_loads() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str("[test]\nscript = \"touch asdf.txt\"\ndesc = \"Create test file\"")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("--help").current_dir(&temp);
    cmd.assert().success().stdout(predicate::str::contains(
        "Commands:\n  test  Create test file\n  help  Print this message or the help of the given subcommand(s)"
    ));

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_from_yaml_loads() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.yaml");
    config_file
        .write_str("test:\n  script: touch asdf.txt\n  desc: Create test file")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("--help").current_dir(&temp);
    cmd.assert().success().stdout(predicate::str::contains(
        "Commands:\n  test  Create test file\n  help  Print this message or the help of the given subcommand(s)"
    ));

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_help_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str("[test]\nscript = \"touch asdf.txt\"\ndesc = \"Create test file\"")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").arg("--help").current_dir(&temp);
    cmd.assert().success().stdout(predicate::str::contains(
        "Create test file\n\nUsage: mach test [OPTIONS]",
    ));

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_command_runs_correctly() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str("[test]\nscript = \"touch asdf.txt\"\ndesc = \"Create test file\"")
        .unwrap();

    let target_file = temp.child("asdf.txt");
    assert!(!target_file.exists());

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").current_dir(&temp);
    cmd.assert().success();
    assert!(target_file.exists());

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_command_executes_multiple_commands() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "echo A\necho B""#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").current_dir(&temp);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A\nB"));

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_tasks_execute_in_correct_order() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "bash script.sh main"
deps = ["test_dep"]
[test_dep]
script = "bash script.sh dep""#,
        )
        .unwrap();

    let script_file = temp.child("script.sh");
    script_file.write_str("echo \"$@\" >> test.txt").unwrap();

    let output_file = temp.child("test.txt");

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").current_dir(&temp);
    cmd.assert().success();

    assert!(output_file.exists());
    output_file.assert(predicate::str::contains("dep\nmain"));

    temp.close().unwrap();
    Ok(())
}
