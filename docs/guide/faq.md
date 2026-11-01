# FAQ

## What exactly is XE?

XE is a small programming language designed for learning and rapid prototyping. It features an indentation-based syntax similar to Python but compiles directly into native Rust source code, which is then built into a standalone executable.

## Why compile to Rust instead of using an interpreter?

Compiling to Rust allows XE to leverage one of the world's most powerful compiler backends (LLVM through rustc). This provides massive performance benefits, exhaustive static analysis, and tiny, standalone binaries without requiring a heavy virtual machine or runtime environment.

## Do I need Rust installed to use XE?

Yes, XE requires the Rust toolchain (specifically rustc) to build your programs into binaries. However, the XE compiler itself handles the heavy lifting of code generation and binary invocation automatically.

## Is XE memory safe?

Because XE code is transpiled into high-level Rust code that follows Rust's ownership and safety rules, the resulting binaries benefit from the memory safety guarantees of the Rust language backend.

## How does the module system work?

XE uses a naming-mangled module system. When you import a module, the compiler assigns it a unique ID and renames its internal symbols to prevent conflicts, allowing for complex multi-file projects without namespace pollution.

## Is this ready for production use?

XE is currently in the "Pre-Alpha" stage. It is an excellent platform for learning about compiler design and language theory, but it is not yet intended for mission-critical production systems.
