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

`starship-worktrunk` reads the `[custom.worktrunk]` section from the same
`starship.toml` file Starship uses. It supports Starship's directory path options
and uses the same defaults when an option is omitted.

To convert an existing directory module:

1. Replace `$directory` with `${custom.worktrunk}` in the top-level `format`.
   The braces are required because the custom module name contains a dot.
2. Rename `[directory]` to `[custom.worktrunk]`.
3. Add `command` and `when`.
4. Change the module `format` to use `$output` instead of the directory-only
   variables such as `$path`.

For example:

```toml
format = """
${custom.worktrunk}\
$git_branch\
$character"""

[custom.worktrunk]
command = "starship-worktrunk"
when = true
format = "[$output]($style) "
style = "bold cyan"
description = "Compacts Worktrunk branch suffixes in repo directory names"

# Existing directory options can remain here.
truncation_length = 3
truncate_to_repo = true
fish_style_pwd_dir_length = 0
use_logical_path = true
truncation_symbol = ""
home_symbol = "~"
use_os_path_sep = true
```

The supported directory path options are:

- `truncation_length`
- `truncate_to_repo`
- `substitutions`, including the list and legacy table forms
- `fish_style_pwd_dir_length`
- `use_logical_path`
- `read_only`
- `truncation_symbol`
- `home_symbol`
- `use_os_path_sep`

The executable finds the configuration using `STARSHIP_CONFIG`, then
`XDG_CONFIG_HOME/starship.toml`, then `~/.config/starship.toml`.

### Formatting limitations

Starship treats a custom command's output as one value, so it cannot apply
different styles to portions of `$output`. `repo_root_style`,
`before_repo_root_style`, `repo_root_format`, and a non-default
`read_only_style` therefore cannot be reproduced faithfully. If these options
are configured, `starship-worktrunk` prints a warning to stderr and uses the
custom module's `style` for the complete output.

The custom module's own `format`, `style`, `disabled`, and `description`
settings continue to be handled by Starship. The executable handles the display
path and read-only symbol.

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
