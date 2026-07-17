use assert_cmd::cargo::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn dry_run_describes_command() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str("[test]\nscript = \"touch asdf.txt\"\ndesc = \"Create test file\"")
        .unwrap();

    let target_file = temp.child("asdf.txt");
    assert!(!target_file.exists());

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test").arg("--dry-run").current_dir(&temp);
    cmd.assert().success().stdout(
        predicate::str::contains("Would execute").and(predicate::str::contains("touch asdf.txt")),
    );
    assert!(!target_file.exists());

    temp.close().unwrap();
    Ok(())
}

#[test]
fn dry_run_expands_info_with_verbose_flag() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary configuration file
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("mach.toml");
    config_file
        .write_str(
            r#"[test]
script = "touch asdf.txt"
desc = "Create test file"
options.environment.MACH_TEST = "as"
"#,
        )
        .unwrap();

    let target_file = temp.child("asdf.txt");
    assert!(!target_file.exists());

    let mut cmd = cargo_bin_cmd!("mach");

    cmd.arg("test")
        .arg("--dry-run")
        .arg("--verbose")
        .current_dir(&temp);

    cmd.assert().success().stdout(
        predicate::str::contains("Would execute")
            .and(predicate::str::contains("touch asdf.txt"))
            .and(predicate::str::contains(
                "Environment variables\x1b[0m\n\t\x1b[1mMACH_TEST\x1b[0m\tas",
            )),
    );
    assert!(!target_file.exists());

    temp.close().unwrap();
    Ok(())
}
