# Types and Values

XE is dynamically typed. Values carry their type at runtime, and operations either succeed according to XE's rules or fail with a runtime error.

## The four runtime types

XE currently has four value kinds:

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | Stored as numeric runtime values |
| `text` | `"hello"` | Double-quoted strings |
| `boolean` | `true`, `false` | Used directly in conditions |
| `list` | `[1, 2, 3]` | Ordered collections with zero-based indexing |

Check a value's runtime type with `type(...)`:

```xe
print(type(42))
print(type("XE"))
print(type([1, 2, 3]))
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
```

Important current rules:

- indexing is zero-based
- negative indexing is not supported
- out-of-bounds access is a runtime error
- list mutation is not implemented yet

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
