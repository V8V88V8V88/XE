# Language Reference

For a complete list of reserved words, see the [Keyword Reference](/guide/keywords).

## Data types

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | 64-bit floating point |
| `text` | `"hello"` | Immutable text; indexing and `length` count characters |
| `boolean` | `true`, `false` | |
| `none` | `none` | The absence of a value |
| `list` | `[1, 2, 3]` | Ordered, mutable, zero-based; negative indexes count from the end |
| `map` | `{"a": 1, 2: "b"}` | Keys are numbers, text or booleans; keeps insertion order |
| `struct` | `Point(1, 2)` | User-defined records with named fields |
| `function` | `double`, `lambda x: x * 2` | Functions are values |

### Shared values

Lists, maps, structs and functions are **shared references**, as in Python. Assigning
one or passing it to a function does not copy it:

```xe
fun fill(items):
    append(items, 99)

a = [1]
b = a
fill(b)
print(a) # [1, 99]
```

Use `copy(value)` for a shallow copy.

### Types and performance

XE infers types. Numbers, text and booleans whose types are known are compiled to
native Rust values (`f64`, `String`, `bool`), including function parameters and return
values when every call agrees on the type. Values whose type can vary are stored
dynamically and checked at runtime.

A variable's type is fixed by its first assignment. Assigning a value of a different
known type later is a compile error; values of unknown type are checked when they are
assigned.

## Operators

| Precedence (low to high) | Operators |
| --- | --- |
| 1 | `or` |
| 2 | `and` |
| 3 | `not` |
| 4 | `==` `!=` `<` `>` `<=` `>=` `in` `not in` |
| 5 | `+` `-` |
| 6 | `*` `/` `//` `%` |
| 7 | unary `-` |
| 8 | `**` (right-associative) |

As in Python, `not a == b` means `not (a == b)` and `-2 ** 2` is `-4`.

- `+` adds numbers, joins text, and concatenates lists. Text joined with any other value converts that value to text.
- `/` is floating-point division; `//` divides and rounds down; `%` takes the sign of the divisor, so `-7 % 3` is `2`.
- `<`, `>`, `<=`, `>=` compare two numbers or two texts (alphabetically).
- `==` compares values deeply; numbers within a tiny relative tolerance are equal, so `0.1 + 0.2 == 0.3`.
- `in` checks list membership, substrings in text, and keys in maps and structs.
- `and`, `or` and `not` accept any value and use its truthiness.

### Truthiness

`false`, `none`, `0`, `""`, `[]` and `{}` are false. Every other value is true. Conditions
in `if`, `elif` and `while` accept any value.

### Assignment

```xe
x = 1
x += 2   # also -=, *=, /=, //=, %=, **=
items[0] = 5
items[-1] += 1
point.x = 3
```

## Indexing and slicing

```xe
items = [10, 20, 30, 40]
print(items[0], items[-1])   # 10 40
print(items[1:3], items[:2]) # [20, 30] [10, 20]
print("hello"[1:-1])         # ell
```

Slices return new lists or texts. Slice bounds outside the sequence are clamped; index
access outside it is a runtime error.

## Control flow

- `if` / `elif` / `else`
- `while condition:`
- `for name in iterable:` over lists, text (characters), maps (keys) and structs (field names)
- `for i in range(n):` (also `range(start, stop)` and `range(start, stop, step)`)
- `repeat N times:`
- `break`, `continue`, `pass`
- `try:` / `catch name:` (see [Errors](#errors))

## Functions

```xe
fun area(width, height):
    return width * height
```

- A function without a `return` value returns `none`.
- `fun`, `fn` and `function` are interchangeable.
- Functions and structs can only be defined at the top level of a module.
- Functions are values: they can be stored in variables, passed to other functions and
  returned from them.

```xe
fun apply(f, items):
    out = []
    for item in items:
        append(out, f(item))
    return out

print(apply(upper, ["a", "b"]))        # ["A", "B"]
print(apply(lambda x: x * 10, [1, 2])) # [10, 20]
```

`lambda params: expression` creates an anonymous function. A lambda can use the
variables around it; their values are copied when the lambda is created.

### Method-call syntax

`value.name(args)` calls `name(value, args)` when `name` is a function, so these are
equivalent:

```xe
print(upper("hi"), "hi".upper())
append(items, 4)
items.append(4)
```

If `name` is not a function but a field holding one, the field is called instead.

### Scope

Scope follows Python:

- A name assigned anywhere in a function is local to that whole function, including
  names first assigned inside `if`, `while` or `try` blocks.
- Reading a local before it has been assigned on every path is a compile error.
- Functions can read module-level variables. To assign one, declare it with `global name`.
- Names assigned at the top level of a module, including inside blocks, are module-level
  variables. A `for` loop variable belongs to its loop.
- Local variables, functions and imports can shadow built-in functions.

## Structs

```xe
struct Point:
    x
    y

p = Point(1, 2)
p.x += 10
print(p)       # Point { x: 11, y: 2 }
```

## Modules and imports

XE modules are other `.xe` files, resolved relative to the importing file
(`import a.b` loads `a/b.xe` or `a/b/index.xe`).

```xe
import geometry              # use geometry.area(...) and geometry.unit
import shapes.circle as c    # use c.area(...)
from geometry import area, unit as one
```

- `import m` binds the name `m`; members are used as `m.name` and can be assigned with `m.x = ...`.
- `from m import name` binds individual functions, structs or variables.
- Imports must appear at the top level, before executable statements.
- A module's top-level code runs once, before the code of the module that imports it.

## Built-in functions

| Function | Purpose |
| --- | --- |
| `print(...)` | Print values separated by spaces |
| `input(prompt)` | Read a line of text (surrounding whitespace removed) |
| `length(value)` | Characters in text, items in a list, entries in a map |
| `type(value)` | Type name as text: `"number"`, `"list"`, ... |
| `convert(value, "number" \| "text" \| "boolean")` | Convert between types |
| `range(stop)`, `range(start, stop, step)` | List of numbers |
| `append(list, item)` | Add to the end; returns the list |
| `insert(list, index, item)` | Insert before `index` |
| `pop(list)`, `pop(list, index)` | Remove and return an item |
| `remove(map, key)` / `remove(list, value)` | Remove a key, or the first matching value |
| `sort(list)`, `reverse(list)` | Sort or reverse in place; return the list |
| `copy(value)` | Shallow copy of a list, map or struct |
| `keys(map)`, `values(map)`, `has_key(map, key)` | Map access |
| `contains(collection, item)` | Same as `item in collection` |
| `split(text)`, `split(text, separator)`, `join(list, separator)` | Text splitting and joining |
| `upper`, `lower`, `trim`, `replace(text, old, new)` | Text transforms |
| `starts_with`, `ends_with`, `find(text, part)` | Text search (`find` returns -1 if absent) |
| `abs`, `floor`, `ceil`, `sqrt`, `round(x)`, `round(x, digits)` | Math (`round` rounds halves away from zero) |
| `min(...)`, `max(...)`, `sum(list)` | `min`/`max` take several values or one list |
| `random()` | Pseudo-random number in [0, 1) |
| `args()` | Command-line arguments as a list of text |
| `exit()`, `exit(code)` | Stop the program |
| `read_file(path)`, `write_file(path, text)`, `append_file(path, text)`, `file_exists(path)` | Files |
| `error(message)` | Raise a runtime error |

## Errors

Compiler errors show the file, line and column, the source line, and a caret.

Runtime errors stop the program and report where they happened:

```text
Runtime error in /path/to/app.xe at line 3: list index 10 out of bounds (length 3)
    print(items[10])
```

`try` / `catch` handles runtime errors. The optional name after `catch` receives the
error message as text:

```xe
try:
    value = convert(text, "number")
catch err:
    print("not a number:", err)
    value = 0
```

## Formatting

- XE uses indentation-based blocks; use spaces for indentation.
- `#` starts a comment.
- Inside a list or map, text is shown quoted: `print(["a", 1])` prints `["a", 1]`.
