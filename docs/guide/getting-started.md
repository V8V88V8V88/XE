# Getting Started

XE depends on the Rust toolchain because the compiler emits Rust and uses `rustc` for final builds.

## Requirements

- `cargo`
- `rustc`
- `git` if you are cloning the repository

## Build the compiler

```bash
git clone https://github.com/V8V88V8V88/XE.git
cd XE
cargo build --release
```

The compiler binary will be available at `./target/release/xe` on Linux and macOS, or `xe.exe` on Windows.

## Run your first XE file

Create `hello.xe`:

```xe
print("Hello, World!")
```

Run it:

```bash
./target/release/xe run hello.xe
```

## Compile a standalone executable

```bash
./target/release/xe compile examples/hello.xe -o hello
./hello
```

## Print generated Rust

If you want to inspect the code XE generates:

```bash
./target/release/xe compile examples/hello.xe
```

## Docs development

This docs site is its own small app inside `docs/`:

```bash
cd docs
npm install
npm run docs:dev
```

Build the docs for deployment:

```bash
npm run docs:build
```
