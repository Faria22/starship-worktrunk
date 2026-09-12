use regex::Regex;
use std::env;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Render with the installed Starship, inheriting its configuration and environment.
pub fn render() -> io::Result<Vec<u8>> {
    let output = Command::new("starship")
        .args(["module", "directory"])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "starship module directory exited with {}",
            output.status
        )));
    }
    let mut rendered = output.stdout;
    if let (Ok(cwd), Ok(text)) = (env::current_dir(), std::str::from_utf8(&rendered))
        && let Some((name, compact)) = worktree_name(&cwd)
        && let Some(result) = compact_rendered(text, &name, &compact)
    {
        rendered = result.into_bytes();
    }
    Ok(rendered)
}

fn git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // Remove Git's line terminator, not whitespace belonging to a path.
    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.strip_suffix('\n').unwrap_or(&s).to_owned())
}

fn worktree_name(cwd: &Path) -> Option<(String, String)> {
    let root = git_output(cwd, &["rev-parse", "--show-toplevel"])?;
    let root = Path::new(&root);
    let branch = git_output(cwd, &["branch", "--show-current"])?;
    if branch.is_empty() {
        return None;
    }
    let name = root.file_name()?.to_str()?;
    let suffix = format!(".{}", branch.replace(['/', '\\'], "-"));
    let compact = name.strip_suffix(&suffix)?;
    if compact.is_empty() || name.chars().any(char::is_control) {
        return None;
    }
    let relative = cwd.strip_prefix(root).ok()?;
    // A truncated child or a visible ancestor could otherwise be mistaken for the root.
    if relative
        .components()
        .any(|part| part.as_os_str().to_string_lossy().contains(name))
        || root
            .parent()?
            .components()
            .any(|part| part.as_os_str().to_string_lossy().contains(name))
    {
        return None;
    }
    Some((name.to_owned(), compact.to_owned()))
}

/// Only replace one intact, delimited name. Keep ANSI styling byte-for-byte.
/// This deliberately declines ambiguous output rather than interpreting Starship config.
fn compact_rendered(rendered: &str, name: &str, compact: &str) -> Option<String> {
    let ansi = Regex::new(r"\x1b\[[0-9;:]*m").expect("valid SGR regex");
    let visible = ansi.replace_all(rendered, "");
    let mut matches = visible.match_indices(name);
    let (start, _) = matches.next()?;
    if matches.next().is_some() || rendered.matches(name).count() != 1 {
        return None;
    }
    let before = visible[..start].chars().next_back();
    let after = visible[start + name.len()..].chars().next();
    let boundary = |c: char| c.is_whitespace() || c == '/' || c == '\\';
    if before.is_some_and(|c| !boundary(c)) || after.is_some_and(|c| !boundary(c) && c != '🔒') {
        return None;
    }
    Some(rendered.replacen(name, compact, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_root_and_path_styles_and_spacing() {
        let text = "\x1b[31mproject.topic\x1b[0m\x1b[36m/src\x1b[0m ";
        assert_eq!(
            compact_rendered(text, "project.topic", "project").unwrap(),
            "\x1b[31mproject\x1b[0m\x1b[36m/src\x1b[0m "
        );
    }

    #[test]
    fn declines_ambiguous_or_transformed_names() {
        for text in [
            "project.topic/project.topic",
            "other-project.topic",
            "project.topic-extra",
            "icon/src",
            "project.\x1b[31mtopic",
        ] {
            assert_eq!(
                compact_rendered(text, "project.topic", "project"),
                None,
                "{text}"
            );
        }
    }

    #[test]
    fn supports_unicode_and_read_only_marker() {
        assert_eq!(
            compact_rendered("项目.topic🔒 ", "项目.topic", "项目").unwrap(),
            "项目🔒 "
        );
    }
}
