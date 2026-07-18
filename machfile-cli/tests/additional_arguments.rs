use assert_cmd::cargo::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn config_command_forwards_arguments_to_first_script() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "echo A\necho B"
options.allow_args = true"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").arg("add").current_dir(&temp);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A add\nB"));

    temp.close().unwrap();
    Ok(())
}

#[test]
fn config_command_executes_without_error_without_additional_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "echo A\necho B"
options.allow_args = true"#,
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
fn additional_arguments_only_gets_forwarded_to_main_task() -> Result<(), Box<dyn std::error::Error>>
{
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "bash script.sh main"
options.allow_args = true
deps = ["test_dep"]
[test_dep]
script = "bash script.sh dep""#,
        )
        .unwrap();

    let script_file = temp.child("script.sh");
    script_file.write_str("echo \"$@\" >> test.txt").unwrap();

    let output_file = temp.child("test.txt");

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").arg("add").current_dir(&temp);
    cmd.assert().success();

    assert!(output_file.exists());
    output_file.assert(predicate::str::contains("dep\nmain add"));

    temp.close().unwrap();
    Ok(())
}
