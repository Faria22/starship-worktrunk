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

fn command_for(path: &Path, home: &Path) -> Command {
    let mut command = Command::new(binary());
    command
        .current_dir(path)
        .env("HOME", home)
        .env_remove("STARSHIP_CONFIG")
        .env_remove("XDG_CONFIG_HOME");
    command
}

fn output_for(path: &Path, home: &Path) -> String {
    let output = command_for(path, home).output().expect("binary should run");

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

#[test]
fn compacts_repo_root_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.worktrunk-support");
    std::fs::create_dir(&repo).expect("repo dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "worktrunk-support"]);

    assert_eq!(output_for(&repo, temp.path()), "starship");
}

#[test]
fn compacts_nested_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.feature-foo");
    let src = repo.join("src");
    std::fs::create_dir_all(&src).expect("nested dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "feature/foo"]);

    assert_eq!(output_for(&src, temp.path()), "starship/src");
}

#[test]
fn keeps_non_matching_git_repo_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let repo = temp.path().join("starship.worktrunk-support");
    std::fs::create_dir(&repo).expect("repo dir should be created");
    git(&repo, &["init"]);
    git(&repo, &["checkout", "-b", "main"]);

    assert_eq!(output_for(&repo, temp.path()), "starship.worktrunk-support");
}

#[test]
fn keeps_non_git_path() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dir = temp.path().join("plain");
    std::fs::create_dir(&dir).expect("dir should be created");

    assert_eq!(output_for(&dir, temp.path()), "~/plain");
}

#[test]
fn reads_directory_options_from_custom_module_config() {
    let temp = TempDir::new().expect("temp dir should be created");
    let dir = temp.path().join("one").join("two");
    std::fs::create_dir_all(&dir).expect("nested dir should be created");
    let config = temp.path().join("starship.toml");
    std::fs::write(
        &config,
        r#"
[custom.worktrunk]
command = "starship-worktrunk"
when = true
format = "[$output]($style) "
style = "bold cyan"
truncation_length = 0
truncate_to_repo = false
home_symbol = "HOME"
"#,
    )
    .expect("config should be written");

    let output = command_for(&dir, temp.path())
        .env("STARSHIP_CONFIG", config)
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .expect("stdout should be utf-8")
            .trim(),
        "HOME/one/two"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn warns_for_unsupported_independent_styles() {
    let temp = TempDir::new().expect("temp dir should be created");
    let config = temp.path().join("starship.toml");
    std::fs::write(
        &config,
        r#"
[custom.worktrunk]
command = "starship-worktrunk"
when = true
format = "[$output]($style) "
repo_root_style = "bold red"
truncation_lenght = 8
"#,
    )
    .expect("config should be written");

    let output = command_for(temp.path(), temp.path())
        .env("STARSHIP_CONFIG", config)
        .output()
        .expect("binary should run");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");

    assert!(output.status.success());
    assert!(stderr.contains("repo_root_style cannot be applied independently"));
    assert!(stderr.contains("unsupported option `truncation_lenght`"));
}
