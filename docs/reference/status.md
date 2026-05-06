# Project Status

XE is **pre-alpha** (current line: `v0.1.2-alpha.7`).

## Works now

- Full pipeline: lexer -> parser -> semantic checks -> Rust codegen -> `rustc`
- CLI: `xe run`, `xe compile`, `xe install`, `xe update`
- Types: `number`, `text`, `boolean`, `list` (+ indexing)
- Control flow: `if/elif/else`, `repeat`, `while`, `for`, `break/continue`
- Functions: `fun`, `return`, recursion
- Built-ins: `print`, `input`, `length`, `type`, `convert`
- Modules: `import x` and `from x import name` across multiple `.xe` files

## Not yet

- List mutation (`items[0] = 42`)
- Constants (`const`)
- Closures / lexical capture
- Module variables usable inside functions
- File I/O + networking + concurrency
- Formatter + debugger + IDE support
