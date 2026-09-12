# starship-worktrunk

`starship-worktrunk` is a small Starship custom module command for people who use
[Worktrunk](https://worktrunk.dev/) branch worktrees.

Worktrunk names branch-specific repo directories with a sanitized branch suffix.
This command keeps your prompt focused on the repo name by removing that suffix
when it matches the current git branch.

## Example

Inside a repo directory named `starship.worktrunk-support` on branch
`worktrunk-support`, the command prints:

```text
starship
```

Inside `starship.feature-foo/src` on branch `feature/foo`, it prints:

```text
starship/src
```

In non-matching repos, detached HEAD states, or directories that are not git
repos, it preserves Starship’s directory output.

## Install

After the package is published to crates.io:

```sh
cargo install starship-worktrunk
```

For local development from this repository:

```sh
cargo install --path .
```

## Starship Configuration

Requires `starship` and `git` on `PATH`. Keep your existing `[directory]` settings
and replace `$directory` with `${custom.worktrunk}` in your top-level `format`
(and `right_format` if applicable). Use an explicit module list if you previously
used `$all`, so the built-in directory is not also displayed.

```toml
format = """
${custom.worktrunk}\
$git_branch\
$git_status\
$character"""

[directory]
style = "bold cyan"
truncation_length = 3
truncate_to_repo = true
read_only = "🔒"
repo_root_style = "bold blue"

[custom.worktrunk]
command = "starship-worktrunk"
when = true
format = "$output "
```

Keep `[directory]` enabled. Its formatting, colors, substitutions, and other
settings are handled by your installed Starship. The custom module passes through
that styled output, so it does not need its own `style`.

The executable invokes `starship module directory` with the existing environment,
including `STARSHIP_CONFIG`. It does not parse configuration or reproduce
Starship's directory defaults. In a matching worktree it removes the branch suffix
from an identifiable repository name in the rendered output.

### Best-effort compaction

Only the default `repo.sanitized-branch` naming convention is supported (`/` and
`\` in branch names become `-`). Detached HEAD, non-matching directories, and
non-Git directories retain Starship's output.

Compaction leaves output unchanged when the name is substituted away, split by
styling, repeated, or not clearly delimited. It also declines compaction when
ancestor or descendant path components contain the same name, since truncation
could otherwise cause a child directory to be mistaken for the repository root.
Arbitrary custom formats are not interpreted; literal text that looks exactly like
a standalone repository name can still be indistinguishable from the real name.

The executable preserves output bytes and adds no newline. Starship custom modules
trim surrounding whitespace; `format = "$output "` restores the usual trailing
space. Adjust that custom format if your prompt needs different spacing.

This adds one Starship process and Git discovery per prompt. Missing or failing
Starship produces an error on stderr and a nonzero exit status.

## Development

Run the project locally:

```sh
cargo run
```

Install Starship before running integration tests (CI uses 1.26.0).
Run the checks used by CI:

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Contributing

Issues and pull requests are welcome.

To contribute:

1. Fork and clone the repository.
2. Create a branch for your change.
3. Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
4. Open a pull request with a short explanation of the behavior change.

Behavior changes should include tests. New dependencies are welcome when they make
the code clearer, safer, or easier to maintain.
