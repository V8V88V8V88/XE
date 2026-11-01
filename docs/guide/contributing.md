# Contributing

XE is open source under GPL-3.0-or-later. Bug reports, examples, documentation fixes and compiler changes are all welcome.

## Reporting issues

Open an issue on [GitHub](https://github.com/V8V88V8V88/XE/issues). For compiler bugs, include:

- the smallest `.xe` program that shows the problem
- what you expected and what happened (copy the full error message)
- the output of `xe --version` and your operating system

## Building from source

You need the [Rust toolchain](https://rustup.rs/) (`rustc` and `cargo`).

```bash
git clone https://github.com/V8V88V8V88/XE.git
cd XE
cargo build --release

# Try it
./target/release/xe run examples/hello.xe
```

## Running the tests

```bash
cargo test
```

CI runs `cargo build --locked` and `cargo test --locked` on every push and pull request.

## How the compiler is laid out

An `.xe` file goes through these stages, each in its own file under `src/`:

| Stage | File | What it does |
| :--- | :--- | :--- |
| Lexing | `lexer.rs` | Turns source text into tokens, including indentation |
| Parsing | `parser.rs`, `ast.rs` | Builds the syntax tree |
| Checking | `semantic.rs` | Resolves names and scopes, infers types |
| Code generation | `codegen.rs` | Emits Rust source |
| Runtime | `runtime/prelude.rs` | Helper code included in every generated program |
| Driver | `compiler.rs`, `main.rs` | Module loading, `rustc` invocation and the `xe` CLI |

Built-in functions are defined in `builtins.rs`, and error types and messages in `error.rs`.

To see the Rust that XE generates for a program, run `xe compile` without `-o`:

```bash
xe compile examples/fib.xe
```

## Working on the docs

This site is built with [VitePress](https://vitepress.dev) from the `docs/` folder:

```bash
cd docs
npm install
npm run docs:dev
```

Every page also has an **Edit this page on GitHub** link at the bottom.
