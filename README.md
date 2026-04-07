<div align="center">
<img src="./XElogo.png" width="120">

# XE Programming Language
</div>

XE is a small programming language that compiles into Rust source code and then into a native executable through `rustc`. Its syntax is indentation-based and intentionally compact so the compiler pipeline stays easy to study.

This is a hobby and learning project. Do not use it in production. It is meant for people who want to see how a simple language and compiler are built.

---

## What is XE?

XE is a **source-to-source** language. That means:

1. You write a `.xe` file (XE source code).
2. The XE compiler reads it and outputs Rust source code.
3. You (or the tooling) run the Rust compiler on that code to get an executable.

So XE does not run your code directly. It translates it to Rust and lets Rust handle the final native build. You get:

- **Simple syntax** – English-like, Python-style, so it is easy to read.
- **Native build output** – XE can produce a standalone executable through Rust.
- **A compact compiler codebase** – lexer, parser, semantic checks, and code generation are all easy to inspect.

---

## Why XE?

XE is primarily a compiler project:

- It gives you a small language to experiment with.
- It shows a complete frontend-to-codegen pipeline.
- It produces native binaries instead of interpreting the source directly.

The main goal is clarity and implementation quality, not feature count.

---

## Benefits

- **Readable syntax** – indentation-based and easy to follow in examples.
- **Native executables** – XE uses `rustc` to build platform binaries.
- **Clear structure** – lexer, parser, semantic analysis, and code generation are separated cleanly.
- **Useful diagnostics** – compiler errors include line/column context and a caret marker.
- **Good for learning** – the codebase is small enough to understand without a large framework.

---

## How it works (the pipeline)

When you run the XE compiler on your source, it follows this transformation process:

```mermaid
graph LR
    Source[".xe Source"] --> Lexer["Lexical Analysis (Tokens)"]
    Lexer --> Parser["Syntactic Analysis (AST)"]
    Parser --> Semantic["Semantic Validation"]
    Semantic --> RustGen["Rust Code Generation"]
    RustGen --> rustc["rustc (Optimization & Native Build)"]
    rustc --> Binary["Native Machine Binary"]
```

1. **Lexer** – Splits your source code into tokens (words, numbers, symbols).
2. **Parser** – Checks that the tokens form valid sentences (syntax) and builds a tree (AST).
3. **AST (Abstract Syntax Tree)** – A tree that represents the structure of your program.
4. **Semantic validation** – Checks names, function calls, loop control, and other structural rules.
5. **Rust code generator** – Writes safe Rust code that does what your XE program says.
6. **Rust compiler (rustc)** – Compiles that Rust code into an executable.

So: XE source → Lexer → Parser → AST → Semantic check → Rust code → rustc → executable.

---

## Documentation

The project now uses a standard docs site inside `docs/`:

- **Main entry:** `docs/index.md`
- **Guides:** Getting started, Language basics, Examples.
- **References:** CLI, Language, Status.

You can run the docs locally:

```bash
cd docs
npm install
npm run docs:dev
```

*(You can also build the docs using `npm run docs:build`.)*

---

## Getting Started

### 1. Prerequisites
You must have the **[Rust toolchain](https://rustup.rs/)** (`cargo` and `rustc`) installed.

### 2. Build the Compiler
```bash
git clone https://github.com/V8V88V8V88/XE.git
cd XE
cargo build --release
```
The compiler binary will be created at `./target/release/xe` (or `xe.exe` on Windows).

*Note: For macOS users, follow the same steps above to build a native binary for Apple Silicon or Intel Macs.*

---

## Usage

### Run an XE program
Compiles and executes the program in one step.
```bash
./target/release/xe run examples/hello.xe
```

### Compile to a Native Binary
Produces a standalone executable.
```bash
./target/release/xe compile examples/hello.xe -o hello
./hello
```

### Print the Generated Rust
Emits the Rust source XE generates to standard output.
```bash
./target/release/xe compile examples/hello.xe
```

---

## Benchmark

The repository includes a reproducible benchmark that compares XE with CPython on the same recursive Fibonacci workload:

```bash
python3 examples/benchmark.py
```

The script:

- builds the XE compiler in release mode if needed
- compiles `examples/benchmark.xe` into a native binary
- runs both implementations multiple times
- prints the mean runtime and the Python/XE speedup ratio

Use the script output for screenshots or performance notes, because the exact numbers depend on your machine.

---

## What you can write in XE

**Data types**

- **Number** – Integers and decimals.
- **Text** – Strings.
- **Boolean** – True and false.
- **List** – An ordered list of values.

XE is currently **dynamically typed**. Values are represented at runtime and operations use XE's coercion rules instead of static type declarations.

**Control flow**

- `if` / `elif` / `else` – Branch on a condition.
- `repeat N times` – Loop a fixed number of times.
- `while` – Loop while a condition stays true.
- `for item in iterable` – Iterate over lists and text.
- `break` / `continue` – Control loop execution.

**Functions**

You can define functions, e.g.:

```xe
function greet(name):
    print("Hello " + name)
```

Functions currently use local scope only. They do not capture outer variables.

**Built-in functions**

- `print()` – Print to the screen.
- `input()` – Read input from the user.
- `length()` – Length of text or a list.
- `type()` – Get the type of a value.
- `convert()` – Convert between `number`, `text`, and `boolean`.

---

## Example programs

**Hello World**

```xe
print("Hello, World!")
```

**Numbers**

```xe
x = 10
y = 20
print(x + y)
```

**Conditional**

```xe
age = input("Enter age:")
age = convert(age, "number")
if age >= 18:
    print("Adult")
elif age >= 13:
    print("Teen")
else:
    print("Minor")
```

---

## What XE does *not* do (for now)

- The feature set is small on purpose.
- No concurrency (no threads, async, etc.).
- No networking or file I/O in the language yet.
- The focus is on doing things correctly, not on fancy optimizations.

Possible future steps: better optimizations (e.g. bytecode or IR), more built-ins, a formatter, debugger, or IDE support.

---

## What you need to run XE

- **Hardware** – A normal computer (e.g. 4 GB RAM or more).
- **Software** – Rust toolchain (so you can compile the generated Rust code), and Git if you clone the repo.

---

## Project Status

**Current Version:** 0.1.1 pre-alpha

The XE compiler is currently in its **Pre-Alpha** stage. The core pipeline works and is covered by integration tests, but the project is still a research prototype.

Recent milestone work completed:

- fixed assignment semantics for reassignment across nested blocks
- added `elif`, `while`, `for`, `break`, and `continue`
- replaced silent runtime fallbacks with explicit runtime errors
- improved compiler errors with source snippets and carets
- added GitHub Actions CI
- expanded integration tests and aligned the docs/CLI with the current implementation

---

## License

This project is licensed under GPL-3.0-or-later - see the [LICENSE](LICENSE) file for details.
