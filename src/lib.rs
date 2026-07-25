use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use unicode_segmentation::UnicodeSegmentation;

const MODULE_NAME: &str = "worktrunk";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitContext {
    pub root: PathBuf,
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderResult {
    pub output: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct DirectoryConfig {
    pub truncation_length: i64,
    pub truncate_to_repo: bool,
    pub substitutions: Substitutions,
    pub fish_style_pwd_dir_length: i64,
    pub use_logical_path: bool,
    pub repo_root_format: String,
    pub repo_root_style: Option<String>,
    pub before_repo_root_style: Option<String>,
    pub read_only: String,
    pub read_only_style: String,
    pub truncation_symbol: String,
    pub home_symbol: String,
    pub use_os_path_sep: bool,
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            truncation_length: 3,
            truncate_to_repo: true,
            substitutions: Substitutions::default(),
            fish_style_pwd_dir_length: 0,
            use_logical_path: true,
            repo_root_format: "[$before_root_path]($before_repo_root_style)[$repo_root]($repo_root_style)[$path]($style)[$read_only]($read_only_style) ".into(),
            repo_root_style: None,
            before_repo_root_style: None,
            read_only: "🔒".into(),
            read_only_style: "red".into(),
            truncation_symbol: String::new(),
            home_symbol: "~".into(),
            use_os_path_sep: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Substitutions {
    List(Vec<Substitution>),
    Table(BTreeMap<String, String>),
}

impl Default for Substitutions {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl Substitutions {
    fn is_empty(&self) -> bool {
        match self {
            Self::List(items) => items.is_empty(),
            Self::Table(items) => items.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Substitution {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub regex: bool,
}

pub fn render() -> RenderResult {
    let cwd = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            return RenderResult {
                output: String::new(),
                warnings: vec![format!(
                    "could not determine the current directory: {error}"
                )],
            };
        }
    };

    let (config, configured_keys, mut warnings) = load_config();
    let git = discover_git_context(&cwd);
    let logical_cwd = logical_directory(&cwd, config.use_logical_path);
    let display_cwd = compacted_path(&logical_cwd, git.as_ref()).unwrap_or(logical_cwd);
    let display_root = git
        .as_ref()
        .and_then(|context| compacted_path(&context.root, Some(context)))
        .or_else(|| git.as_ref().map(|context| context.root.clone()));

    warnings.extend(unsupported_option_warnings(&configured_keys));
    let output = render_directory_path(
        &display_cwd,
        &cwd,
        display_root.as_deref(),
        &config,
        &mut warnings,
    );

    RenderResult { output, warnings }
}

pub fn display_path() -> String {
    render().output
}

fn load_config() -> (DirectoryConfig, BTreeSet<String>, Vec<String>) {
    let Some(path) = starship_config_path() else {
        return (DirectoryConfig::default(), BTreeSet::new(), Vec::new());
    };
    if !path.exists() {
        return (DirectoryConfig::default(), BTreeSet::new(), Vec::new());
    }

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            return (
                DirectoryConfig::default(),
                BTreeSet::new(),
                vec![format!("could not read {}: {error}", path.display())],
            );
        }
    };
    let document = match toml::from_str::<toml::Value>(&source) {
        Ok(document) => document,
        Err(error) => {
            return (
                DirectoryConfig::default(),
                BTreeSet::new(),
                vec![format!("could not parse {}: {error}", path.display())],
            );
        }
    };

    let Some(module) = document
        .get("custom")
        .and_then(|custom| custom.get(MODULE_NAME))
        .cloned()
    else {
        return (DirectoryConfig::default(), BTreeSet::new(), Vec::new());
    };

    let configured_keys: BTreeSet<_> = module
        .as_table()
        .into_iter()
        .flat_map(|table| table.keys().cloned())
        .collect();
    let mut warnings = unknown_option_warnings(&configured_keys);
    match module.try_into::<DirectoryConfig>() {
        Ok(config) => (config, configured_keys, warnings),
        Err(error) => {
            warnings.push(format!(
                "could not load [custom.{MODULE_NAME}] from {}: {error}",
                path.display()
            ));
            (DirectoryConfig::default(), configured_keys, warnings)
        }
    }
}

fn starship_config_path() -> Option<PathBuf> {
    env::var_os("STARSHIP_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .map(|path| path.join("starship.toml"))
        })
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|path| path.join(".config").join("starship.toml"))
        })
}

fn unknown_option_warnings(configured_keys: &BTreeSet<String>) -> Vec<String> {
    const KNOWN_OPTIONS: &[&str] = &[
        "before_repo_root_style",
        "command",
        "description",
        "detect_extensions",
        "detect_files",
        "detect_folders",
        "disabled",
        "fish_style_pwd_dir_length",
        "format",
        "home_symbol",
        "ignore_timeout",
        "os",
        "read_only",
        "read_only_style",
        "repo_root_format",
        "repo_root_style",
        "require_repo",
        "shell",
        "style",
        "substitutions",
        "symbol",
        "truncate_to_repo",
        "truncation_length",
        "truncation_symbol",
        "unsafe_no_escape",
        "use_logical_path",
        "use_os_path_sep",
        "use_stdin",
        "when",
    ];
    configured_keys
        .iter()
        .filter(|key| !KNOWN_OPTIONS.contains(&key.as_str()))
        .map(|key| format!("unsupported option `{key}` in [custom.{MODULE_NAME}] is ignored"))
        .collect()
}

fn unsupported_option_warnings(configured_keys: &BTreeSet<String>) -> Vec<String> {
    let mut warnings = Vec::new();
    if configured_keys.contains("repo_root_style") {
        warnings.push(
            "repo_root_style cannot be applied independently by a Starship custom module; the custom module style applies to the entire output".into(),
        );
    }
    if configured_keys.contains("before_repo_root_style") {
        warnings.push(
            "before_repo_root_style cannot be applied independently by a Starship custom module; the custom module style applies to the entire output".into(),
        );
    }
    if configured_keys.contains("repo_root_format") {
        warnings.push(
            "repo_root_format cannot be evaluated by a Starship custom module and is ignored"
                .into(),
        );
    }
    if configured_keys.contains("read_only_style") {
        warnings.push(
            "read_only_style cannot be applied independently by a Starship custom module; the custom module style applies to the entire output".into(),
        );
    }
    warnings
}

fn logical_directory(physical: &Path, enabled: bool) -> PathBuf {
    if !enabled {
        return physical.to_path_buf();
    }
    env::var_os("PWD")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && paths_refer_to_same_location(path, physical))
        .unwrap_or_else(|| physical.to_path_buf())
}

fn paths_refer_to_same_location(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn render_directory_path(
    display_cwd: &Path,
    physical_cwd: &Path,
    repo_root: Option<&Path>,
    config: &DirectoryConfig,
    warnings: &mut Vec<String>,
) -> String {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|path| path.canonicalize().unwrap_or(path));
    let repo_contracted = if config.truncate_to_repo {
        repo_root.and_then(|root| contract_repo_path(display_cwd, root))
    } else {
        None
    };
    let mut is_truncated = repo_contracted.is_some();
    let contracted = repo_contracted.unwrap_or_else(|| {
        home.as_deref()
            .map(|home| contract_path(display_cwd, home, &config.home_symbol))
            .unwrap_or_else(|| slash_path(display_cwd))
    });

    let substituted = match substitute_path(contracted.clone(), &config.substitutions) {
        Ok(path) => path,
        Err(error) => {
            warnings.push(format!("invalid regex in substitutions: {error}"));
            contracted
        }
    };
    let (truncated, did_truncate) = truncate_path(&substituted, config.truncation_length);
    is_truncated |= did_truncate;

    let prefix = if is_truncated {
        if config.fish_style_pwd_dir_length > 0 && config.substitutions.is_empty() {
            let full = home
                .as_deref()
                .map(|home| contract_path(display_cwd, home, &config.home_symbol))
                .unwrap_or_else(|| slash_path(display_cwd));
            to_fish_style(config.fish_style_pwd_dir_length as usize, &full, &truncated)
        } else {
            config.truncation_symbol.clone()
        }
    } else {
        String::new()
    };

    let mut output = prefix + &truncated;
    if config.use_os_path_sep && std::path::MAIN_SEPARATOR != '/' {
        output = output.replace('/', std::path::MAIN_SEPARATOR_STR);
    }
    if is_read_only(physical_cwd) {
        output.push_str(&config.read_only);
    }
    output
}

fn compacted_path(cwd: &Path, git: Option<&GitContext>) -> Option<PathBuf> {
    let git = git?;
    let suffix = format!(".{}", sanitize_branch_name(&git.branch));
    let repo_name = git.root.file_name()?.to_string_lossy();
    let compacted_repo_name = repo_name.strip_suffix(&suffix)?;
    if compacted_repo_name.is_empty() {
        return None;
    }

    let relative = cwd.strip_prefix(&git.root).ok()?;
    let mut compacted = git.root.clone();
    compacted.set_file_name(compacted_repo_name);
    if !relative.as_os_str().is_empty() {
        compacted.push(relative);
    }
    Some(compacted)
}

pub fn display_path_for(cwd: &Path, git: Option<&GitContext>) -> String {
    compacted_path(cwd, git)
        .unwrap_or_else(|| cwd.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

pub fn compact_worktrunk_path(cwd: &Path, repo_root: &Path, branch: &str) -> Option<String> {
    compacted_path(
        cwd,
        Some(&GitContext {
            root: repo_root.to_path_buf(),
            branch: branch.to_owned(),
        }),
    )
    .map(|path| path.to_string_lossy().into_owned())
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
    String::from_utf8(output.stdout)
        .ok()
        .map(|output| output.trim().to_owned())
}

fn contract_repo_path(full_path: &Path, repo_root: &Path) -> Option<String> {
    let relative = full_path.strip_prefix(repo_root).ok()?;
    let name = repo_root.file_name()?.to_string_lossy();
    if relative.as_os_str().is_empty() {
        Some(name.into_owned())
    } else {
        Some(format!("{name}/{}", slash_path(relative)))
    }
}

fn contract_path(full_path: &Path, home: &Path, symbol: &str) -> String {
    if full_path == home {
        return symbol.to_owned();
    }
    match full_path.strip_prefix(home) {
        Ok(relative) => format!("{symbol}/{}", slash_path(relative)),
        Err(_) => slash_path(full_path),
    }
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn substitute_path(
    mut path: String,
    substitutions: &Substitutions,
) -> Result<String, regex::Error> {
    match substitutions {
        Substitutions::List(items) => {
            for item in items {
                path = if item.regex {
                    Regex::new(&item.from)?
                        .replace(&path, &item.to)
                        .into_owned()
                } else {
                    path.replace(&item.from, &item.to)
                };
            }
        }
        Substitutions::Table(items) => {
            for (from, to) in items {
                path = path.replace(from, to);
            }
        }
    }
    Ok(path)
}

fn truncate_path(path: &str, length: i64) -> (String, bool) {
    if length <= 0 {
        return (path.to_owned(), false);
    }
    let mut parts: Vec<_> = path.split('/').collect();
    if parts.first() == Some(&"") {
        parts.remove(0);
    }
    let length = length as usize;
    if parts.len() <= length {
        return (path.to_owned(), false);
    }
    (parts[parts.len() - length..].join("/"), true)
}

fn to_fish_style(length: usize, full: &str, truncated: &str) -> String {
    full.trim_end_matches(truncated)
        .split('/')
        .map(|component| {
            let graphemes: Vec<_> = component.graphemes(true).collect();
            if component.is_empty() || graphemes.len() <= length {
                component.to_owned()
            } else if component.starts_with('.') {
                graphemes[..=(length.min(graphemes.len() - 1))].concat()
            } else {
                graphemes[..length].concat()
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_read_only(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> PathBuf {
        parts.iter().collect()
    }

    #[test]
    fn directory_defaults_match_starship() {
        let config = DirectoryConfig::default();
        assert_eq!(config.truncation_length, 3);
        assert!(config.truncate_to_repo);
        assert_eq!(config.fish_style_pwd_dir_length, 0);
        assert!(config.use_logical_path);
        assert_eq!(config.read_only, "🔒");
        assert_eq!(config.truncation_symbol, "");
        assert_eq!(config.home_symbol, "~");
        assert!(config.use_os_path_sep);
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
        assert_eq!(
            actual,
            Some(path(&["tmp", "starship"]).display().to_string())
        );
    }

    #[test]
    fn compacts_nested_path() {
        let repo_root = path(&["tmp", "starship.feature-foo"]);
        let cwd = repo_root.join("src").join("module");
        let actual = compact_worktrunk_path(&cwd, &repo_root, "feature/foo");
        assert_eq!(
            actual,
            Some(
                path(&["tmp", "starship", "src", "module"])
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn applies_truncation_and_substitution() {
        let config = DirectoryConfig {
            truncation_length: 2,
            truncation_symbol: "…/".into(),
            substitutions: Substitutions::List(vec![Substitution {
                from: "module".into(),
                to: "mod".into(),
                regex: false,
            }]),
            ..DirectoryConfig::default()
        };
        let mut warnings = Vec::new();
        let output = render_directory_path(
            Path::new("/tmp/project/src/module"),
            Path::new("/tmp/project/src/module"),
            None,
            &config,
            &mut warnings,
        );
        assert_eq!(output, "…/src/mod");
        assert!(warnings.is_empty());
    }

    #[test]
    fn fish_style_preserves_dot_prefix() {
        assert_eq!(
            to_fish_style(
                1,
                "~/.starship/engines/booster/rocket",
                "engines/booster/rocket"
            ),
            "~/.s/"
        );
    }

    #[test]
    fn root_component_does_not_count_toward_truncation_length() {
        assert_eq!(
            truncate_path("/starship/engines/booster", 3),
            ("/starship/engines/booster".into(), false)
        );
        assert_eq!(
            truncate_path("/starship/engines/booster/rocket", 3),
            ("engines/booster/rocket".into(), true)
        );
    }
}
