use assert_cmd::Command;
use std::env;
use std::process::{Command as StdCommand, Stdio};
use tempfile::tempdir;

#[test]
fn test_scanner_with_example_wheel() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let wheel_path = temp_dir
        .path()
        .join("example_package-0.1.0-py3-none-any.whl");

    let project_root = env::current_dir()?
        .parent()
        .ok_or("Failed to find parent directory")?
        .to_path_buf();

    let example_dir = project_root
        .join("wheel-metadata-injector")
        .join("examples");

    unsafe {
        env::set_var("TEST_ENV_VAR", "test_value");
        env::set_var("WHEEL_SCANNER_TEST", "running_e2e_test");
    }

    let output = StdCommand::new("python3")
        .args([
            "setup.py",
            "bdist_wheel",
            "--env-vars",
            "TEST_ENV_VAR,WHEEL_SCANNER_TEST,PATH",
        ])
        .current_dir(&example_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build wheel: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let dist_dir = example_dir.join("dist");
    let wheel_files = std::fs::read_dir(&dist_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.is_file()
                && path.extension().map_or(false, |ext| ext == "whl")
                && path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("example_package")
        })
        .collect::<Vec<_>>();

    if wheel_files.is_empty() {
        return Err("No wheel file found in dist directory".into());
    }

    let latest_wheel = wheel_files
        .into_iter()
        .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap())
        .unwrap();

    if !latest_wheel.path().exists() {
        return Err("No wheel file found in dist directory".into());
    }

    std::fs::copy(latest_wheel.path(), &wheel_path)?;

    let mut cmd = Command::cargo_bin("scanner")?;
    let output = cmd
        .arg(wheel_path.to_string_lossy().to_string())
        .assert()
        .success()
        .get_output()
        .to_owned();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("TEST_ENV_VAR=test_value"),
        "TEST_ENV_VAR not found in output"
    );
    assert!(
        stdout.contains("WHEEL_SCANNER_TEST=running_e2e_test"),
        "WHEEL_SCANNER_TEST not found in output"
    );
    assert!(stdout.contains("Found dist-info directory:"));
    assert!(stdout.contains("Found WHEEL.metadata"));
    assert!(stdout.contains("=== Build Environment Metadata ==="));

    temp_dir.close()?;

    Ok(())
}
