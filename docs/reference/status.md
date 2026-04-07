# Project Status

## Current state

XE is pre-alpha. The full compiler pipeline is present:

- lexer
- parser
- semantic analysis
- code generation
- CLI entrypoint

## Recently completed

- fixed assignment semantics so existing variables are reassigned instead of accidentally shadowed
- added `elif`
- added `while`
- added `for item in iterable`
- added `break` and `continue`
- replaced silent runtime fallbacks with explicit runtime errors for invalid operations
- improved compiler diagnostics with source-line and caret rendering
- added GitHub Actions CI for build-and-test checks
- expanded integration coverage for control flow, scoping, and CLI behavior
- aligned the docs and CLI help text with actual compiler behavior

## Implemented language features

- `print(...)`
- `input(prompt)`
- `length(value)`
- `type(value)`
- `convert(value, target_type)`
- `if` / `elif` / `else`
- `repeat N times`
- `while`
- `for item in iterable`
- `break` / `continue`
- user-defined functions
- recursion
- list literals and indexing
- arithmetic, comparison, and boolean operators
- dynamic runtime typing with semantic checks for names, arity, and loop control

Current public release line: `v0.1.1`

## Planned work

- assignment into list elements like `items[0] = 42`
- `else` blocks for loops if you want Python-style control flow
- more built-in functions for everyday programs
- file I/O
- networking
- concurrency
- modules and `import`
- formatter
- debugger
- IDE support
- richer built-ins like `floor`, `ceil`, `min`, and `max`
