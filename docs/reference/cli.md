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

## `install`

- Copies the current XE binary into a local bin directory
- Default install target is `~/.local/bin`
- `--to <directory>` lets you choose a custom install directory
- After installation, that directory must be in your shell `PATH` to run `xe` directly

## `help`

Shows command usage.
