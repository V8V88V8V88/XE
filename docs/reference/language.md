# Language Reference

## Data types

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | Numeric values |
| `text` | `"hello"` | String values |
| `boolean` | `true`, `false` | Condition values |
| `list` | `[1, 2, 3]` | Ordered values with zero-based indexing |

XE is currently dynamically typed. Variables do not carry declared static types, and values use runtime coercions where the language allows them.

Invalid operations now fail with explicit runtime errors instead of silently producing fallback values.

## Operators

### Arithmetic

- `+`
- `-`
- `*`
- `/`
- `%`

### Comparison

- `==`
- `!=`
- `<`
- `>`
- `<=`
- `>=`

### Logic

- `and`
- `or`
- `not`

## Built-in functions

| Function | Purpose | Example |
| --- | --- | --- |
| `print(...)` | Print one or more values | `print("hi")` |
| `input(prompt)` | Read text from the user | `name = input("Name? ")` |
| `length(value)` | Return length of text or list | `length([1, 2, 3])` |
| `type(value)` | Return the XE type name as text | `type(42)` |
| `convert(value, target)` | Convert between supported types | `convert("42", "number")` |

Valid `convert` targets:

- `"number"`
- `"text"`
- `"boolean"`

## Control flow

- `if`
- `elif`
- `else`
- `repeat N times`
- `while`
- `for name in iterable`
- `break`
- `continue`

`for` loops currently iterate over:

- lists
- text values, one character at a time

## Function syntax

```xe
fun name(arg1, arg2):
    return arg1 + arg2
```

Functions currently use their own local scope. They can access:

- parameters
- variables created inside the function
- built-in functions
- other user-defined functions
- imported user-defined functions

They do not capture outer variables from surrounding scopes.

## Modules and imports

XE modules are other `.xe` files.

```xe
from math_utils import double
print(double(21))
```

Available forms:

- `import helpers`
- `from helpers import square, cube`

Current rules:

- imports resolve relative to the importing file
- `import module_name` brings that module's top-level functions into scope
- `from module_name import name` imports selected top-level functions
- only top-level functions are importable today
- imports must appear before executable top-level statements
- imported module top-level code runs once before the entry module's top-level code

Example project shape:

- `main.xe`
- `helpers.xe`

## Notes on formatting

- XE uses indentation-based blocks
- spaces are the expected style for indentation
- there are no braces for `if`, `elif`, `else`, `repeat`, `while`, `for`, or `fun` blocks

## Assignment semantics

- `name = value` creates a variable if that name does not exist in any enclosing scope
- otherwise, it reassigns the nearest existing variable instead of creating a shadow copy
- names first created inside a block stay local to that block

## Error behavior

- compiler errors include line/column information, the source line, and a caret marker
- invalid runtime operations stop the program with a `Runtime error: ...` message
