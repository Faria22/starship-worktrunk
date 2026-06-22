use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitContext {
    pub root: PathBuf,
    pub branch: String,
}

pub fn display_path() -> String {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(_) => return String::new(),
    };

    display_path_for(&cwd, discover_git_context(&cwd).as_ref())
}

pub fn display_path_for(cwd: &Path, git: Option<&GitContext>) -> String {
    let Some(git) = git else {
        return path_to_string(cwd);
    };

    compact_worktrunk_path(cwd, &git.root, &git.branch).unwrap_or_else(|| path_to_string(cwd))
}

pub fn compact_worktrunk_path(cwd: &Path, repo_root: &Path, branch: &str) -> Option<String> {
    let suffix = format!(".{}", sanitize_branch_name(branch));
    let repo_name = repo_root.file_name()?.to_string_lossy();
    let compacted_repo_name = repo_name.strip_suffix(&suffix)?;

    if compacted_repo_name.is_empty() {
        return None;
    }

    let relative_path = cwd.strip_prefix(repo_root).ok()?;
    let mut compacted_path = repo_root.to_path_buf();
    compacted_path.set_file_name(compacted_repo_name);
    if !relative_path.as_os_str().is_empty() {
        compacted_path.push(relative_path);
    }

    Some(path_to_string(&compacted_path))
}

pub fn sanitize_branch_name(branch: &str) -> String {
    branch.replace(['/', '\\'], "-")
}

fn discover_git_context(cwd: &Path) -> Option<GitContext> {
    let root = git_output(cwd, ["rev-parse", "--show-toplevel"])?;

    let branch = match git_output(cwd, ["branch", "--show-current"]) {
        Some(branch) if !branch.is_empty() => branch,
        _ => git_output(cwd, ["rev-parse", "--abbrev-ref", "HEAD"])
            .filter(|branch| !branch.is_empty() && branch != "HEAD")?,
    };

    Some(GitContext {
        root: PathBuf::from(root),
        branch,
    })
}

fn git_output<const N: usize, S>(cwd: &Path, args: [S; N]) -> Option<String>
where
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    Some(stdout.trim().to_owned())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> PathBuf {
        parts.iter().collect()
    }

    #[test]
    fn sanitizes_branch_name() {
        assert_eq!(sanitize_branch_name("feature/foo"), "feature-foo");
        assert_eq!(sanitize_branch_name(r"feature\foo"), "feature-foo");
        assert_eq!(sanitize_branch_name("bug.fix_123"), "bug.fix_123");
    }

    #[test]
    fn compacts_matching_repo_suffix() {
        let repo_root = path(&["tmp", "starship.worktrunk-support"]);
        let actual = compact_worktrunk_path(&repo_root, &repo_root, "worktrunk-support");

        assert_eq!(actual, Some(path_to_string(&path(&["tmp", "starship"]))));
    }

    #[test]
    fn compacts_nested_path() {
        let repo_root = path(&["tmp", "starship.feature-foo"]);
        let cwd = repo_root.join("src").join("module");
        let actual = compact_worktrunk_path(&cwd, &repo_root, "feature/foo");

        assert_eq!(
            actual,
            Some(path_to_string(&path(&["tmp", "starship", "src", "module"])))
        );
    }

    #[test]
    fn does_not_compact_non_matching_suffix() {
        let repo_root = path(&["tmp", "starship.worktrunk-support"]);
        let actual = compact_worktrunk_path(&repo_root, &repo_root, "other-branch");

        assert_eq!(actual, None);
    }

    #[test]
    fn does_not_compact_branch_name_prefix() {
        let repo_root = path(&["tmp", "starship.worktrunk-support-extra"]);
        let actual = compact_worktrunk_path(&repo_root, &repo_root, "worktrunk-support");

        assert_eq!(actual, None);
    }

    #[test]
    fn display_path_falls_back_without_git_context() {
        let cwd = path(&["tmp", "starship.worktrunk-support"]);

        assert_eq!(display_path_for(&cwd, None), path_to_string(&cwd));
    }
}
