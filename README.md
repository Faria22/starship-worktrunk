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
/path/to/starship
```

Inside `starship.feature-foo/src` on branch `feature/foo`, it prints:

```text
/path/to/starship/src
```

In non-matching repos, detached HEAD states, or directories that are not git
repos, it prints the current directory unchanged.

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

Use this as a replacement for Starship's built-in directory module:

```toml
[directory]
disabled = true

[custom.worktrunk]
command = "starship-worktrunk"
when = true
style = "bold cyan" # Or the value from your [directory] section.
format = "[$output]($style) " # Or the value from your [directory] section, with $path replaced by $output.
description = "Compacts Worktrunk branch suffixes in repo directory names"
```

Starship handles styling. This command only prints the display path.

## Development

Run the project locally:

```sh
cargo run
```

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
