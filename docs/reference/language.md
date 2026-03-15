# Language Reference

## Data types

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | Numeric values |
| `text` | `"hello"` | String values |
| `boolean` | `true`, `false` | Condition values |
| `list` | `[1, 2, 3]` | Ordered values with zero-based indexing |

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
- `else`
- `repeat N times`

## Function syntax

```xe
function name(arg1, arg2):
    return arg1 + arg2
```

## Notes on formatting

- XE uses indentation-based blocks
- spaces are the expected style for indentation
- there are no braces for `if`, `else`, `repeat`, or `function` blocks
