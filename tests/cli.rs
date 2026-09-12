use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(command: &mut Command) -> Output {
    let output = command
        .output()
        .expect("command should run (Starship must be installed)");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

struct Fixture {
    _temp: TempDir,
    repo: PathBuf,
    config: PathBuf,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let temp = TempDir::new().unwrap();
        let repo = temp.path().join("project.feature-foo");
        std::fs::create_dir(&repo).unwrap();
        run(Command::new("git").args(["init", "-q"]).arg(&repo));
        run(Command::new("git")
            .current_dir(&repo)
            .args(["checkout", "-qb", "feature/foo"]));
        let config_path = temp.path().join("starship.toml");
        std::fs::write(&config_path, config).unwrap();
        Self {
            _temp: temp,
            repo,
            config: config_path,
        }
    }

    fn command(&self, binary: &str, path: &Path) -> Command {
        let mut cmd = Command::new(binary);
        cmd.current_dir(path)
            .env("PWD", path)
            .env("STARSHIP_CONFIG", &self.config)
            .env("STARSHIP_SHELL", if cfg!(windows) { "cmd" } else { "sh" })
            .env_remove("NO_COLOR")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE");
        cmd
    }

    fn compare(&self, path: &Path, compact: bool) {
        let expected = run(self.command("starship", path).args(["module", "directory"])).stdout;
        let expected = String::from_utf8(expected).unwrap();
        let expected = if compact {
            assert!(
                expected.contains("project.feature-foo"),
                "probe must exercise compaction"
            );
            expected.replacen("project.feature-foo", "project", 1)
        } else {
            expected
        };
        let actual = run(&mut self.command(env!("CARGO_BIN_EXE_starship-worktrunk"), path));
        assert!(
            actual.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&actual.stderr)
        );
        assert_eq!(actual.stdout, expected.as_bytes());
    }
}

#[test]
fn delegates_directory_settings_and_preserves_ansi() {
    for options in [
        "",
        "repo_root_style = 'bold red'",
        "truncate_to_repo = false\ntruncation_length = 0",
        "use_logical_path = false",
        "format = '[$path]($style)  '",
        "substitutions = [{from='src',to='source'}]",
    ] {
        let fixture = Fixture::new(&format!("[directory]\n{options}\n"));
        fixture.compare(&fixture.repo, true);
        let src = fixture.repo.join("src");
        std::fs::create_dir(&src).unwrap();
        fixture.compare(&src, true);
    }
}

#[test]
fn delegates_fish_style_shortening_independently_of_shell() {
    let mut fixture = Fixture::new(
        "[directory]\nfish_style_pwd_dir_length = 1\nformat = '$path'\nuse_os_path_sep = false\n",
    );
    let home = fixture._temp.path().canonicalize().unwrap();
    let parent = home.join("development").join("projects");
    std::fs::create_dir_all(&parent).unwrap();
    let repo = parent.join("project.feature-foo");
    std::fs::rename(&fixture.repo, &repo).unwrap();
    fixture.repo = repo;
    let src = fixture.repo.join("src");
    std::fs::create_dir(&src).unwrap();

    // Fish-style shortening is a directory setting, not a fish-only shell feature.
    // Calling `module directory` does not launch the selected shell.
    for shell in [if cfg!(windows) { "cmd" } else { "sh" }, "fish"] {
        for path in [&fixture.repo, &src] {
            let suffix = if path == &src { "/src" } else { "" };
            for (binary, name) in [
                ("starship", "project.feature-foo"),
                (env!("CARGO_BIN_EXE_starship-worktrunk"), "project"),
            ] {
                let mut command = fixture.command(binary, path);
                command
                    .env("STARSHIP_SHELL", shell)
                    .env("HOME", &home)
                    .env("USERPROFILE", &home);
                if binary == "starship" {
                    command.args(["module", "directory"]);
                }
                let output = run(&mut command);
                assert!(
                    output.stderr.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    output.stdout,
                    format!("~/d/p/{name}{suffix}").as_bytes(),
                    "{binary} with STARSHIP_SHELL={shell}",
                );
            }
        }
    }
}

#[test]
fn preserves_substituted_output() {
    let fixture =
        Fixture::new("[directory]\nsubstitutions = [{from='project.feature-foo',to='icon'}]\n");
    fixture.compare(&fixture.repo, false);
}

#[test]
fn preserves_hidden_root_and_identical_child() {
    let fixture = Fixture::new("[directory]\ntruncation_length = 1\n");
    for name in ["src", "project.feature-foo"] {
        let child = fixture.repo.join(name);
        std::fs::create_dir(&child).unwrap();
        fixture.compare(&child, false);
    }
}

#[test]
fn preserves_non_matching_branch_and_non_git_directory() {
    let fixture = Fixture::new("");
    run(Command::new("git")
        .current_dir(&fixture.repo)
        .args(["checkout", "-qb", "main"]));
    fixture.compare(&fixture.repo, false);
    fixture.compare(fixture._temp.path(), false);
}

#[test]
fn preserves_detached_head() {
    let fixture = Fixture::new("");
    run(Command::new("git").current_dir(&fixture.repo).args([
        "-c",
        "user.name=Test",
        "-c",
        "user.email=test@example.com",
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "initial",
    ]));
    run(Command::new("git")
        .current_dir(&fixture.repo)
        .args(["checkout", "--detach"]));
    fixture.compare(&fixture.repo, false);
}

#[test]
fn custom_module_round_trip_keeps_styles() {
    let fixture = Fixture::new(&format!(
        // Allow debug builds under parallel test load to exceed the prompt default.
        r#"
command_timeout = 5000
[directory]
repo_root_style = 'bold red'
[custom.worktrunk]
command = '"{}"'
when = true
format = '$output '
"#,
        env!("CARGO_BIN_EXE_starship-worktrunk")
    ));
    let direct = run(fixture
        .command("starship", &fixture.repo)
        .args(["module", "directory"]));
    let expected =
        String::from_utf8(direct.stdout)
            .unwrap()
            .replacen("project.feature-foo", "project", 1);
    let nested = run(fixture
        .command("starship", &fixture.repo)
        .args(["module", "custom.worktrunk"]));
    assert_eq!(
        nested.stdout,
        expected.as_bytes(),
        "{}",
        String::from_utf8_lossy(&nested.stderr)
    );
}

#[cfg(unix)]
#[test]
fn reports_missing_or_failing_starship() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new("");
    let bin = fixture._temp.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let invoke = || {
        fixture
            .command(env!("CARGO_BIN_EXE_starship-worktrunk"), &fixture.repo)
            .env("PATH", &bin)
            .output()
            .unwrap()
    };
    let missing = invoke();
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    let fake = bin.join("starship");
    std::fs::write(&fake, "#!/bin/sh\necho failed >&2\nexit 7\n").unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    let failed = invoke();
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("failed"));
}
