# Runtime Behavior and Errors

XE is intentionally strict about invalid operations. Earlier versions used silent fallbacks in a few places, but the current language runtime stops with explicit errors instead.

## Two error categories

You will usually see one of two kinds of failures:

1. compiler errors
2. runtime errors

Compiler errors happen before code generation succeeds.

Runtime errors happen when the compiled program runs and reaches an invalid operation.

## Compiler errors

Compiler errors include:

- undefined variables
- undefined functions
- wrong argument counts
- missing modules
- missing imported names
- circular imports
- invalid indentation
- `break` outside loops
- `continue` outside loops
- `return` outside functions

The compiler prints:

- line and column
- the source line
- a caret pointing at the location

Example shape:

```text
Error at line 1, column 7: undefined variable 'name'
print(name)
      ^
```

## Runtime errors

Runtime errors currently cover cases like:

- division by zero
- modulo by zero
- invalid `convert(...)` operations
- invalid `length(...)` usage
- invalid index access
- invalid `repeat` counts
- iterating with `for` over unsupported values
- numeric comparisons on non-number values

Example:

```xe
print(convert("abc", "number"))
```

That stops with a runtime error instead of silently producing `0`.

## Indexing rules

Index access works on:

- lists
- text

```xe
items = [10, 20]
print(items[1])
print("XE"[0])
```

Current rules:

- the index must be a non-negative integer
- out-of-bounds access is a runtime error
- negative indexes are not supported

## Conversion rules and failure cases

`convert(...)` is intentionally narrow.

Examples that work:

```xe
print(convert("42", "number"))
print(convert(1, "boolean"))
print(convert(true, "text"))
```

Examples that fail:

```xe
print(convert("abc", "number"))
print(convert([1, 2], "number"))
```

## Current limitations worth knowing

XE is usable for small examples, but it still has deliberate limits:

- no list mutation
- no constants
- no file I/O or networking in the language
- no static type checker
- no closures
- functions still do not capture top-level variables

Knowing these limits makes it easier to write correct XE programs and to explain the language honestly.

## Debugging advice

When something goes wrong:

1. start with `xe compile file.xe` and inspect the generated Rust if needed
2. reduce the program to the smallest failing example
3. check scope boundaries first, especially inside functions and blocks
4. check whether a conversion or index access is failing at runtime

## Next steps

- Revisit [Language Basics](/guide/language-basics) for a quicker overview
- Use [Examples](/guide/examples) for runnable programs
