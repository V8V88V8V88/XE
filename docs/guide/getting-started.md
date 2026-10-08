# Getting Started

XE is an expressive, indentation-based programming language that translates into optimized Rust and compiles directly into standalone native machine binaries using `rustc`.

---

## System Prerequisites

Because XE generates Rust code and invokes `rustc` with native optimizations (`-C opt-level=3`), you need the **Rust toolchain** (`rustc` and `cargo`) installed on your system.

::: code-group

```bash [Linux (Ubuntu / Debian)]
# 1. Install build essentials and curl
sudo apt update && sudo apt install -y curl git build-essential

# 2. Install Rust toolchain via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

```bash [Linux (Fedora / RHEL)]
# 1. Install build tools and curl
sudo dnf groupinstall -y "Development Tools" && sudo dnf install -y curl git

# 2. Install Rust toolchain via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

```bash [Linux (Arch Linux)]
# Install base-devel, git, and Rust
sudo pacman -S --needed base-devel git rustup
rustup default stable
```

```bash [macOS (Apple Silicon & Intel)]
# 1. Install Xcode Command Line Tools
xcode-select --install

# 2. Install Rust toolchain via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

```bash [FreeBSD]
# Install Rust, Git, and Curl via pkg
pkg install -y rust git curl
```

```powershell [Windows (PowerShell)]
# Install Rust toolchain using winget
winget install Rustlang.Rustup

# Alternatively, download and run the installer from:
# https://rustup.rs/
```

:::

---

## Installation {#quick-install}

Choose the installation method that matches your operating system:

### 1. One-Line Script Install (Recommended for Unix)

::: code-group

```bash [Linux]
curl -fsSL https://xe-lang.vercel.app/install.sh | bash
```

```bash [macOS]
curl -fsSL https://xe-lang.vercel.app/install.sh | bash
```

```bash [FreeBSD]
curl -fsSL https://xe-lang.vercel.app/install.sh | bash
```

:::

> **Custom directory or specific version:**
> ```bash
> # Install to a custom directory (e.g. ~/bin)
> XE_INSTALL_DIR="$HOME/bin" curl -fsSL https://xe-lang.vercel.app/install.sh | bash
>
> # Install a specific release version
> XE_VERSION="v0.2.0" curl -fsSL https://xe-lang.vercel.app/install.sh | bash
> ```

---

### 2. Windows Installation

::: code-group

```powershell [Via GitHub Releases (Zip)]
# 1. Download the latest release zip for Windows:
#    https://github.com/v8v88v8v88/XE/releases/latest
# 2. Extract xe.exe to your preferred directory (e.g. %LOCALAPPDATA%\Programs\XE\bin)
# 3. Add that directory to your PATH environment variable.
```

```powershell [Via Cargo]
# If you have Rust/Cargo installed:
cargo install --git https://github.com/V8V88V8V88/XE --bin xe
```

:::

---

### 3. Build from Source (All Platforms)

To build the compiler directly from the Git repository:

::: code-group

```bash [Linux / macOS / FreeBSD]
# 1. Clone repository
git clone https://github.com/V8V88V8V88/XE.git
cd XE

# 2. Build in release mode
cargo build --release

# 3. Install binary to ~/.local/bin
./target/release/xe install
```

```powershell [Windows (PowerShell)]
# 1. Clone repository
git clone https://github.com/V8V88V8V88/XE.git
cd XE

# 2. Build in release mode
cargo build --release

# 3. Install binary
.\target\release\xe.exe install
```

:::

---

## Verifying Installation

Verify that the XE compiler is available on your PATH:

```bash
xe --version
```
Expected output:
```text
xe version 0.2.0
```

> **PATH Troubleshooting:** If `xe` is not recognized after installation:
> - **Bash / Zsh:** Ensure `export PATH="$HOME/.local/bin:$PATH"` is present in `~/.bashrc` or `~/.zshrc`.
> - **Fish:** Run `set -Ux fish_user_paths $HOME/.local/bin $fish_user_paths`.
> - **Windows:** Check that `%LOCALAPPDATA%\Programs\XE\bin` or your target folder is in your User Environment Variables PATH.

---

## Running Your First Program

### 1. Create a Source File

Create a file named `hello.xe`:

```xe
# hello.xe
fun greet(name):
    print("Welcome to XE,", name + "!")

greet("World")
```

### 2. Run Directly with `xe run`

Compiles the program and runs it in one step:

```bash
xe run hello.xe
```
Output:
```text
Welcome to XE, World!
```

### 3. Compile to a Standalone Binary with `xe compile -o`

Builds an optimized, standalone native binary:

```bash
# Linux / macOS / FreeBSD
xe compile hello.xe -o hello
./hello

# Windows
xe compile hello.xe -o hello.exe
.\hello.exe
```

### 4. Inspect Generated Rust Code

If you want to view the typed Rust code produced by the XE compiler:

```bash
xe compile hello.xe
```

---

## Keeping XE Updated

XE has built-in self-updating. Whenever a new release is published on GitHub, simply run:

```bash
xe update
```
