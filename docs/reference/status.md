# Project Status

XE is in **pre-alpha** (v0.2.0). While the compiler is stable enough for experimentation and learning, it is not yet intended for production use.

## Current Capabilities

The compiler provides a complete pipeline from `.xe` source to a native binary, using the Rust toolchain as its backend.

*   **Native Execution**: XE translates source code into Rust, which is then compiled into a standalone executable. Programs run directly on host hardware without the overhead of an interpreter.
*   **Indentation-Based Syntax**: Uses Python-style indentation for block structure, focusing on readability and a compact code footprint.
*   **Type Inference**: Numbers, text and booleans with known types, including function parameters and return values, compile to native Rust values (`f64`, `String`, `bool`). Values whose type can vary are stored dynamically and checked at runtime.
*   **Module System**: Projects can be organized across multiple files using `import` and `from ... import`. The compiler resolves the dependency graph and links all modules into a single binary.
*   **Scoped State**: Functions can read their parameters, local variables, and module-level globals, and assign to globals declared with `global`.
*   **Standard Control Flow**: `if/elif/else`, `while`, `for` (over lists, text, maps and `range`), `repeat N times`, and `try/catch` error handling.
*   **Functions as Values**: Functions can be passed around and returned, and `lambda` creates anonymous functions.
*   **Developer Interface**: The `xe` CLI manages the full workflow: `xe run` for rapid testing, `xe compile` for optimized builds, and `xe install` to manage the environment.

## New in v0.2.0

*   **Python-style Values and Scope**: Lists, maps and structs are shared references; variables assigned inside `if`/loop blocks stay visible after them; reading a variable that might not be assigned is a compile error.
*   **Faster Programs**: Variable reads no longer copy lists, function parameters and return values get native types when calls agree, and `for i in range(n)` is a native loop. Summing a 20,000-item list by index went from 7.6 s to under 0.01 s, and `fib(32)` from 0.17 s to 0.013 s.
*   **New Syntax**: `none`, `pass`, `try`/`catch`, `lambda`, `+=` and friends, `**`, `//`, `in` / `not in`, slicing (`items[1:3]`), negative indexes, method-call syntax (`items.append(4)`), and `import module as name`.
*   **Namespaced Imports**: `import m` binds `m`; use `m.name`. (`from m import name` is unchanged.)
*   **Built-ins**: `range`, text functions (`upper`, `lower`, `trim`, `replace`, `find`, ...), math functions (`abs`, `round`, `sqrt`, `min`, `max`, `sum`, ...), `sort`, `reverse`, `insert`, `remove`, `copy`, `random`, `args`, `exit`, file I/O and `error`.
*   **Better Maps**: Keys can be numbers, text or booleans, and maps keep insertion order.
*   **Clearer Errors**: Runtime errors show the file, line and source line; syntax errors show the source line; internal names no longer appear in messages. `not` now binds like in Python (`not a == b`), conditions accept any value, and comparisons work on text.
*   **CLI**: `xe run` caches compiled programs (a repeat run takes milliseconds) and passes extra arguments to the program. XE now asks before installing Rust or editing your shell profile.

## New in v0.1.5

*   **`global` Keyword**: Functions can assign to module-level variables with `global name`. Reading a name before assigning it in the same function is now a clear compile error.
*   **Reliable Globals**: Functions always see the current value of globals, and changes made inside functions (including `append`) are kept.
*   **`none` Return Value**: Functions that return no value now give `none` instead of `0`.
*   **Nested Assignment**: `grid[0][1] = 5`, `p.pos.x = 1` and `append(grid[0], x)` update the original data.
*   **Fewer Rust Errors**: Several cases where valid XE code produced Rust compile errors are fixed, such as reassigning parameters and variables named like Rust keywords.
*   **Text & Number Fixes**: `length()` counts characters instead of bytes, and very large numbers print correctly.
*   **FreeBSD Builds**: Prebuilt FreeBSD binaries and installer support. Building XE no longer needs OpenSSL.

## New in v0.1.4

*   **List Equality & Robust Comparisons**: Full deep comparison support for lists (`==` / `!=`) via `xe_eq`.
*   **String & List Indexing**: Correct typed unwrapping for indexed expressions (`.as_string()`, `.as_f64()`, etc.).
*   **Nested List Iteration**: Full support for iterating over 2D and nested lists in `for` loops.
*   **Graceful Bounds Checking**: Native list indexing bounds check reporting clean runtime errors instead of panics.
*   **AST Scoping & Variable Shadowing**: Robust multi-depth scope management ensuring local loop variables and function parameters shadow module-level symbols properly.
*   **Safe Intermediate Compilation**: Temporary files in `compile -o` are safely generated in OS temp directories.

## Current Limitations

The following areas are currently under development or not yet implemented:

*   **Nested Definitions**: Functions and structs must be defined at the top level; use `lambda` for inline functions. Lambdas copy the variables they use when they are created.
*   **Standard Library**: There is no networking, and no list comprehensions yet.
*   **Concurrency**: There is currently no support for threads or asynchronous execution.
