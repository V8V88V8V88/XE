# CLI Reference

## Commands

```text
xe compile <file.xe>
xe compile <file.xe> -o <output>
xe install [--to <directory>]
xe run <file.xe>
xe help
```

## `compile`

- Without `-o`, prints generated Rust code to standard output
- With `-o`, writes temporary Rust code and then invokes `rustc` to create a native executable
- `compile -o` produces a binary, not a saved `.rs` source file
- Extra arguments are rejected instead of being ignored

## `run`

- Compiles the XE file
- Builds a temporary executable with `rustc`
- Runs the program immediately

## Rust toolchain

`run` and `compile -o` need `rustc`. If it is missing, XE asks before installing it with rustup; in a non-interactive shell it prints instructions instead. Set `XE_INSTALL_RUST=1` to allow the installation without asking.

## `install`

- Copies the current XE binary into a local bin directory
- Default install target is `~/.local/bin`
- `--to <directory>` lets you choose a custom install directory
- If the directory is not on your `PATH`, XE asks before adding it to your shell profile (or your user `PATH` on Windows); in a non-interactive shell it only prints instructions

## `help`

Shows command usage.
