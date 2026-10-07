# Types and Values

XE uses an **inferred type system** with a **Typed IR**. The compiler tries to map variables directly to native Rust types for performance, falling back to a dynamic `XeValue` box when types are mixed or unknown.

## Value kinds

XE has six primary runtime value kinds:

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | Stored as numeric runtime values |
| `text` | `"hello"` | Double-quoted strings |
| `boolean` | `true`, `false` | Used directly in conditions |
| `list` | `[1, 2, 3]` | Ordered collections with zero-based indexing and mutation |
| `map` | `{"a": 1, "b": 2}` | Key-value dictionary collections |
| `struct` | `Point(10, 20)` | User-defined composite structures with named fields |

Check a value's runtime type with `type(...)`:

```xe
print(type(42))
print(type("XE"))
print(type([1, 2, 3]))
print(type({"a": 1}))
```

## Variables are mutable

XE currently has variables, not constants.

```xe
score = 10
score = score + 5
print(score)
```

There is no separate `const` keyword right now. Reassignment is allowed unless the name is out of scope.

## Numbers

Numbers support:

- `+`
- `-`
- `*`
- `/`
- `%`

```xe
price = 99.5
tax = 0.5
print(price + tax)
```

Division and modulo by zero raise runtime errors instead of silently returning fallback values.

## Text

Text values use double quotes.

```xe
name = "XE"
print("Hello " + name)
```

`+` also performs concatenation if either operand is text.

```xe
print("Count: " + 3)
```

That produces text output.

## Booleans and truthiness

Boolean literals are:

- `true`
- `false`

Conditions also use truthiness rules:

- `0` is falsey
- non-zero numbers are truthy
- `""` is falsey
- non-empty text is truthy
- `[]` is falsey
- non-empty lists are truthy

Example:

```xe
if [1, 2]:
    print("non-empty lists are truthy")
```

## Lists

Lists are array-like collections backed by Rust `Vec`, so indexing is natural and efficient.

```xe
items = [10, 20, 30]
print(items[0])
print(length(items))

# Index mutation
items[1] = 99
print(items) # [10, 99, 30]

# Collection operations
append(items, 40)
print(items) # [10, 99, 30, 40]

last = pop(items)
print(last) # 40

print(contains(items, 99)) # true
```

String utility functions:

```xe
words = split("cat,dog,bird", ",")
print(words[0], words[1]) # cat dog

joined = join(words, "-")
print(joined) # cat-dog-bird
```

## Maps (Dictionaries)

Maps store key-value associations with text keys:

```xe
user = {
    "name": "Alice",
    "age": 30,
    "admin": true
}

# Key access & mutation
print(user["name"])
user["age"] = 31

# Map built-ins
print(has_key(user, "name")) # true
ks = keys(user)
vs = values(user)

# Map iteration (iterates over keys)
for k in user:
    print(k, user[k])
```

## Structs

User-defined structs provide typed composite data modeling with named fields:

```xe
struct Point:
    x
    y

# Constructor instantiation
p = Point(10, 20)

# Field access and mutation
print(p.x, p.y) # 10 20
p.x = 42
print(p.x, p.y) # 42 20

# Value-based structural equality
p1 = Point(1, 2)
p2 = Point(1, 2)
print(p1 == p2) # true
```

## Conversion rules

`convert(value, target)` supports three targets:

- `"number"`
- `"text"`
- `"boolean"`

Examples:

```xe
print(convert("42", "number"))
print(convert(123, "text"))
print(convert("", "boolean"))
```

Current behavior:

- text to number must parse successfully
- invalid text-to-number conversion fails at runtime
- booleans convert to `1` or `0` when targeting `"number"`
- lists cannot be converted to numbers

## Equality and comparison

Comparison operators:

- `==`
- `!=`
- `<`
- `>`
- `<=`
- `>=`

Ordering comparisons (`<`, `>`, `<=`, `>=`) are numeric comparisons. If you use them on non-number values, XE raises a runtime error.

Equality compares values of the same runtime kind naturally. Different runtime kinds are not treated as equal.

```xe
print(5 == 5)
print("xe" == "xe")
print(true == 1)
```

The last line evaluates to `false`, not to an implicit coercion.

## Next steps

- Continue with [Control Flow](/guide/control-flow)
- Or jump to [Functions and Scope](/guide/functions-and-scope)
