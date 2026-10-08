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
- reading a local variable that might not be assigned yet
- type mismatches the compiler can prove, such as `5 + "a" * 2`

The compiler prints:

- the file, line and column
- the source line
- a caret pointing at the location

Example shape:

```text
Error in /path/to/app.xe at line 1, column 7: undefined variable 'name'
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
- comparing values that cannot be ordered, such as a number and text
- reading a module-level variable before it has been assigned
- calling `error("message")`

Example:

```xe
print(convert("abc", "number"))
```

That stops with a runtime error instead of silently producing `0`. The message says where it happened:

```text
Runtime error in /path/to/app.xe at line 1: cannot convert text 'abc' to number
    print(convert("abc", "number"))
```

## Handling errors with `try` / `catch`

```xe
fun parse_number(text):
    try:
        return convert(text, "number")
    catch err:
        print("not a number:", err)
        return 0

print(parse_number("42"), parse_number("abc"))
```

The name after `catch` is optional and receives the error message as text. `return`, `break` and `continue` work inside both blocks.

## Indexing rules

Index access works on lists and text, and on maps by key:

```xe
items = [10, 20]
print(items[1], items[-1])
print("XE"[0])
```

Rules:

- the index must be a whole number
- negative indexes count from the end: `items[-1]` is the last item
- out-of-bounds access is a runtime error; slices such as `items[1:10]` are clamped instead

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

- no constants
- no networking, threads or async
- no list comprehensions
- functions and structs must be defined at the top level (use `lambda` inside functions)
- lambdas copy the variables they use when they are created

Knowing these limits makes it easier to write correct XE programs and to explain the language honestly.

## Debugging advice

When something goes wrong:

1. start with `xe compile file.xe` and inspect the generated Rust if needed
2. reduce the program to the smallest failing example
3. check scope first: a name assigned anywhere in a function is local to it unless declared `global`
4. check whether a conversion or index access is failing at runtime

## Next steps

- Revisit [Language Basics](/guide/language-basics) for a quicker overview
- Use [Examples](/guide/examples) for runnable programs
