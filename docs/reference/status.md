# Project Status

XE is in **pre-alpha** (v0.1.4). While the compiler is stable enough for experimentation and learning, it is not yet intended for production use.

## Current Capabilities

The compiler provides a complete pipeline from `.xe` source to a native binary, using the Rust toolchain as its backend.

*   **Native Execution**: XE translates source code into Rust, which is then compiled into a standalone executable. Programs run directly on host hardware without the overhead of an interpreter.
*   **Indentation-Based Syntax**: Uses Python-style indentation for block structure, focusing on readability and a compact code footprint.
*   **Typed IR & Inference**: Through a "Hybrid IR," the compiler maps variables to native Rust types (`f64`, `bool`, `String`) for performance, only falling back to boxed values (`XeValue`) when types are ambiguous or mixed.
*   **Module System**: Projects can be organized across multiple files using `import` and `from ... import`. The compiler resolves the dependency graph and links all modules into a single binary.
*   **Scoped State**: Functions can read and write their parameters, local variables, and module-level globals.
*   **Standard Control Flow**: Full support for `if/elif/else` branches, `while` loops, `for` loops (iterating over lists or text), and `repeat N times` loops.
*   **Developer Interface**: The `xe` CLI manages the full workflow: `xe run` for rapid testing, `xe compile` for optimized builds, and `xe install` to manage the environment.

## New in v0.1.4

*   **List Equality & Robust Comparisons**: Full deep comparison support for lists (`==` / `!=`) via `xe_eq`.
*   **String & List Indexing**: Correct typed unwrapping for indexed expressions (`.as_string()`, `.as_f64()`, etc.).
*   **Nested List Iteration**: Full support for iterating over 2D and nested lists in `for` loops.
*   **Graceful Bounds Checking**: Native list indexing bounds check reporting clean runtime errors instead of panics.
*   **AST Scoping & Variable Shadowing**: Robust multi-depth scope management ensuring local loop variables and function parameters shadow module-level symbols properly.
*   **Safe Intermediate Compilation**: Temporary files in `compile -o` are safely generated in OS temp directories.

## Current Limitations

The following areas are currently under development or not yet implemented:

*   **Immutable Lists**: You can create and read lists, but mutating individual elements (e.g., `items[0] = 42`) is not yet supported.
*   **No Nested Closures**: Functions can access module-level globals but cannot yet capture local variables from an outer function scope.
*   **Standard Library**: The built-in suite is limited to basic I/O, type conversion, and length checks. Networking and file system access are not yet available.
*   **Concurrency**: There is currently no support for threads or asynchronous execution.
