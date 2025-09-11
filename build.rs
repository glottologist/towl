use std::process::Command;

fn run_command<const N: usize>(command: &str, args: [&str; N]) -> String {
    let output = Command::new(command).args(args).output().expect("failed to get run command");
    String::from_utf8(output.stdout).expect("invalid command output")
}

fn git_hash() -> String {
    run_command("git", ["rev-parse", "HEAD"])
}

fn main() {
    let hash = git_hash();
    println!("cargo:rustc-env=TOWL_GIT_COMMIT_HASH={hash}");
}

