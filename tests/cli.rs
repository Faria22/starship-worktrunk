use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn run(command: &mut Command) {
    let output = command.output().expect("command should run");
    assert!(
        output.status.success(),
        "command failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(path: &Path, args: &[&str]) {
    run(Command::new("git").args(args).current_dir(path));
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_starship-worktrunk")
}

fn output_for(path: &Path) -> String {
    let output = Command::new(binary())
        .current_dir(path)
        .output()
        .expect("binary should run");

    assert!(
        output.status.success(),
        "binary failed with status {}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("stdout should be utf-8")
        .trim()
        .to_owned()
}

fn expected(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .canonicalize()
        .expect("expected path should canonicalize")
        .display()
        .to_string()
}

fn expected_under(base: &Path, parts: &[&str]) -> String {
    let mut path = base
        .canonicalize()
        .expect("expected base path should canonicalize");
    path.extend(parts);
    path.display().to_string()
}

#[test]
fn compacts_repo_root_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.worktrunk-support");
    std::fs::create_dir(&repo).expect("repo dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "worktrunk-support"]);

    assert_eq!(
        output_for(&repo),
        expected_under(temp.path(), &["starship"])
    );
}

#[test]
fn compacts_nested_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.feature-foo");
    let src = repo.join("src");
    std::fs::create_dir_all(&src).expect("nested dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "feature/foo"]);

    assert_eq!(
        output_for(&src),
        expected_under(temp.path(), &["starship", "src"])
    );
}

#[test]
fn keeps_non_matching_git_repo_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.worktrunk-support");
    std::fs::create_dir(&repo).expect("repo dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "main"]);

    assert_eq!(output_for(&repo), expected(&repo));
}

#[test]
fn keeps_non_git_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dir = temp.path().join("plain");
    std::fs::create_dir(&dir).expect("dir should be created");

    assert_eq!(output_for(&dir), expected(&dir));
}
