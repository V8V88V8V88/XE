# CLI Reference

## Commands

```text
xe compile <file.xe>
xe compile <file.xe> -o <output>
xe run <file.xe>
xe help
```

## `compile`

- Without `-o`, prints generated Rust code to standard output
- With `-o`, writes temporary Rust code and then invokes `rustc` to create a native executable

## `run`

- Compiles the XE file
- Builds a temporary executable with `rustc`
- Runs the program immediately

## `help`

Shows command usage.
