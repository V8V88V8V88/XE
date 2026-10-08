# CLI Reference

## Commands

```text
xe compile <file.xe>
xe compile <file.xe> -o <output>
xe install [--to <directory>]
xe run <file.xe> [arguments...]
xe update
xe help
```

## `compile`

- Without `-o`, prints generated Rust code to standard output
- With `-o`, writes temporary Rust code in a private temporary directory and then invokes `rustc` to create a native executable
- `compile -o` produces a binary, not a saved `.rs` source file
- Extra arguments are rejected instead of being ignored

## `run`

- Compiles the XE file and runs it immediately
- Arguments after the file name are passed to the program (read them with `args()`)
- The program's exit code becomes `xe run`'s exit code
- Compiled programs are cached, so running unchanged code again skips `rustc`. The cache lives in `$XE_CACHE_DIR`, or `~/.cache/xe` (`%LOCALAPPDATA%\xe\cache` on Windows), and keeps the 50 most recent builds

## Rust toolchain

`run` and `compile -o` need `rustc`. If it is missing, XE asks before installing it with rustup; in a non-interactive shell it prints instructions instead. Set `XE_INSTALL_RUST=1` to allow the installation without asking.

## `install`

- Copies the current XE binary into a local bin directory
- Default install target is `~/.local/bin`
- `--to <directory>` lets you choose a custom install directory
- If the directory is not on your `PATH`, XE asks before adding it to your shell profile (or your user `PATH` on Windows); in a non-interactive shell it only prints instructions

## `update`

- Downloads and installs the latest XE release from GitHub

## `help`

Shows command usage.
