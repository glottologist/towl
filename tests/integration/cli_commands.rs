use assert_cmd::Command;
use predicates::prelude::*;
use rstest::*;
use std::fs;
use tempfile::TempDir;

#[fixture]
fn test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create test files with TODO comments
    let rust_file = temp_dir.path().join("test.rs");
    fs::write(
        &rust_file,
        r#"
// TODO: implement this function
fn example() {
    // FIXME: handle error case
    println!("test");
    // HACK: temporary workaround
}
"#,
    )
    .expect("Failed to write test file");

    let js_file = temp_dir.path().join("test.js");
    fs::write(
        &js_file,
        r#"
// TODO: add validation
function test() {
    // NOTE: this is important
    console.log("test");
}
"#,
    )
    .expect("Failed to write test file");

    temp_dir
}

#[rstest]
fn test_scan_command_basic(test_project: TempDir) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--format")
        .arg("terminal");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("implement this func"))
        .stdout(predicate::str::contains("handle error case"));
}

#[rstest]
fn test_scan_table_format(test_project: TempDir) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--format")
        .arg("table");

    cmd.assert().success();
}

#[rstest]
#[case("json", "json")]
#[case("csv", "csv")]
#[case("markdown", "md")]
#[case("toml", "toml")]
fn test_scan_file_output_formats(
    test_project: TempDir,
    #[case] format: &str,
    #[case] extension: &str,
) {
    let output_file = test_project.path().join(format!("output.{extension}"));

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--format")
        .arg(format)
        .arg("--output")
        .arg(&output_file);

    cmd.assert().success();
    assert!(output_file.exists());
}

#[rstest]
#[case("todo", "implement this func", "handle error case")]
#[case("fixme", "handle error case", "implement this func")]
#[case("hack", "temporary workaround", "implement this func")]
fn test_todo_type_filtering(
    test_project: TempDir,
    #[case] todo_type: &str,
    #[case] should_contain: &str,
    #[case] should_not_contain: &str,
) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--todo-type")
        .arg(todo_type)
        .arg("--format")
        .arg("terminal");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(should_contain))
        .stdout(predicate::str::contains(should_not_contain).not());
}

#[rstest]
fn test_verbose_flag(test_project: TempDir) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--verbose")
        .arg("--format")
        .arg("terminal");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Found"))
        .stderr(predicate::str::contains("TODO comments"));
}

#[rstest]
fn test_init_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("towl.toml");

    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("init").arg("--path").arg(&config_path);

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Initialized config file"));

    assert!(config_path.exists());
}

#[rstest]
fn test_config_command() {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("config");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("config"));
}

#[rstest]
fn test_nonexistent_path() {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg("/nonexistent/path")
        .arg("--format")
        .arg("terminal");

    cmd.assert().failure().code(1);
}

#[rstest]
fn test_invalid_todo_type(test_project: TempDir) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--todo-type")
        .arg("INVALID_TYPE")
        .arg("--format")
        .arg("terminal");

    cmd.assert().failure();
}

#[rstest]
fn test_case_insensitive_filtering(test_project: TempDir) {
    let mut cmd = Command::cargo_bin("towl").unwrap();
    cmd.arg("scan")
        .arg(test_project.path())
        .arg("--todo-type")
        .arg("todo") // lowercase
        .arg("--format")
        .arg("terminal");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TODO"));
}
