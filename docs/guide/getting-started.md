# Getting Started

XE can be installed either from a published release binary or by building from source.

## Quick install {#quick-install}

The fastest path on macOS and Linux is the hosted install script:

```bash
curl -fsSL https://xe-lang.vercel.app/install.sh | bash
```

This downloads the latest XE binary from GitHub Releases and installs it into `~/.local/bin` by default.

Useful variants:

```bash
XE_INSTALL_DIR="$HOME/bin" curl -fsSL https://xe-lang.vercel.app/install.sh | bash
XE_VERSION="v0.1.1" curl -fsSL https://xe-lang.vercel.app/install.sh | bash
```

If you are on Windows, use the release zip from GitHub Releases for now.

Release binaries are published from the public GitHub repository, so the installer expects public release assets.

## Build from source

XE depends on the Rust toolchain because the compiler emits Rust and uses `rustc` for final builds.

Requirements:

- `cargo`
- `rustc`
- `git` if you are cloning the repository

Build the compiler:

```bash
git clone https://github.com/V8V88V8V88/XE.git
cd XE
cargo build --release
```

The compiler binary will be available in Cargo's release output directory.

Install that built binary into `~/.local/bin`:

```bash
./target/release/xe install
```

If you want to build and install in one step from the repository checkout:

```bash
cargo run --release -- install
```

If `xe` is still not found after installation, add the install directory to your shell `PATH`.

Examples:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

```fish
set -Ux fish_user_paths $HOME/.local/bin $fish_user_paths
```

## Install through Cargo

If you prefer to install from the repository through Cargo:

```bash
cargo install --git https://github.com/V8V88V8V88/XE --bin xe
```

## Run your first XE file

Create `hello.xe`:

```xe
print("Hello, World!")
```

Run it:

```bash
xe run hello.xe
```

## Compile a standalone executable

```bash
xe compile examples/hello.xe -o hello
./hello
```

## Print generated Rust

If you want to inspect the code XE generates:

```bash
xe compile examples/hello.xe
```

If you use `-o`, XE will build a native executable instead of saving the Rust source:

```bash
xe compile examples/hello.xe -o hello
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
