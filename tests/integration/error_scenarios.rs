use assert_cmd::Command;
use predicates::prelude::*;
use rstest::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

#[rstest]
fn test_init_existing_config_without_force() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("towl.toml");

    fs::write(&config_path, "[parsing]\nfile_extensions = [\"rs\"]")
        .expect("Failed to write config");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("init").arg("--path").arg(&config_path);

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[rstest]
fn test_init_existing_config_with_force() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("towl.toml");

    fs::write(&config_path, "[parsing]\nfile_extensions = [\"rs\"]")
        .expect("Failed to write config");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("init")
        .arg("--path")
        .arg(&config_path)
        .arg("--force");

    cmd.assert()
        .success()
        .stderr(predicates::str::contains("Initialized config file"));
}

#[rstest]
fn test_output_to_readonly_location() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a source file with todos
    let source_file = temp_dir.path().join("test.rs");
    fs::write(&source_file, "// TODO: test").expect("Failed to write source file");

    // Try to output to a location that doesn't exist or is not writable
    let output_path = "/proc/invalid_output.json";

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(output_path);

    // scan_todos swallows save errors and returns Ok with a warning
    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Failed to save output"));
}

#[rstest]
fn test_invalid_output_format() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let source_file = temp_dir.path().join("test.rs");
    fs::write(&source_file, "// TODO: test").expect("Failed to write source file");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("invalid_format");

    cmd.assert().failure(); // Should fail with invalid format
}

#[rstest]
fn test_config_command_with_missing_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.current_dir(temp_dir.path()).arg("config");

    cmd.assert()
        .success()
        .stderr(predicates::str::contains("Towl Configuration"));
}

#[rstest]
fn test_scan_with_permission_denied_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create a file and make it unreadable
    let test_file = temp_dir.path().join("restricted.rs");
    fs::write(&test_file, "// TODO: test").expect("Failed to write test file");

    let mut perms = fs::metadata(&test_file).unwrap().permissions();
    perms.set_mode(0o000); // No permissions
    fs::set_permissions(&test_file, perms).ok();

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("terminal");

    // Should handle permission errors gracefully (skips unreadable files)
    cmd.assert().success();
}

#[rstest]
fn test_scan_deeply_nested_directories() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create deeply nested directory structure
    let mut deep_path = temp_dir.path().to_path_buf();
    for i in 0..20 {
        deep_path = deep_path.join(format!("level_{i}"));
    }
    fs::create_dir_all(&deep_path).expect("Failed to create deep directories");

    // Add a file at the deepest level
    let deep_file = deep_path.join("deep.rs");
    fs::write(&deep_file, "// TODO: deep test").expect("Failed to write deep file");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("terminal");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}

#[rstest]
fn test_output_to_directory_instead_of_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let source_file = temp_dir.path().join("test.rs");
    fs::write(&source_file, "// TODO: test").expect("Failed to write source file");

    // Try to output to a directory path instead of a file
    let output_dir = temp_dir.path().join("output_dir");
    fs::create_dir(&output_dir).expect("Failed to create output directory");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(temp_dir.path())
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&output_dir);

    // Output path is a directory without extension, so Output::new fails
    cmd.assert().failure().code(1);
}

#[rstest]
fn test_concurrent_scans() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let source_file = temp_dir.path().join("test.rs");
    fs::write(&source_file, "// TODO: test").expect("Failed to write source file");

    // Run multiple scans simultaneously (basic concurrency test)
    let mut handles = vec![];

    for i in 0..3 {
        let output_file = temp_dir.path().join(format!("output_{i}.json"));
        let temp_path = temp_dir.path().to_path_buf();

        let handle = std::thread::spawn(move || {
            let mut cmd = Command::cargo_bin("towl").unwrap();
            cmd.arg("scan")
                .arg(&temp_path)
                .arg("--format")
                .arg("json")
                .arg("--output")
                .arg(&output_file);

            cmd.assert().success();
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}

#[rstest]
fn test_malformed_command_line_args() {
    // Test various malformed command line arguments
    let test_cases = vec![
        vec!["scan", "--format"],       // Missing format value
        vec!["unknown_command"],        // Invalid command
        vec!["scan", "--invalid-flag"], // Invalid flag
    ];

    for args in test_cases {
        let mut cmd = Command::cargo_bin("towl").unwrap();
        for arg in args {
            cmd.arg(arg);
        }

        // Should fail gracefully with proper error message
        cmd.assert().failure();
    }
}
