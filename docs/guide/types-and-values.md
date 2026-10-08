# Types and Values

XE infers types. Numbers, text and booleans whose types are known are compiled to native Rust values (`f64`, `String`, `bool`); values whose type can vary are stored dynamically and checked at runtime.

## Value kinds

| Type | Example | Notes |
| --- | --- | --- |
| `number` | `42`, `3.14` | 64-bit floating point |
| `text` | `"hello"` | Double-quoted; immutable |
| `boolean` | `true`, `false` | |
| `none` | `none` | The absence of a value |
| `list` | `[1, 2, 3]` | Ordered, mutable, zero-based |
| `map` | `{"a": 1, "b": 2}` | Key-value pairs in insertion order |
| `struct` | `Point(10, 20)` | User-defined records with named fields |
| `function` | `print`, `lambda x: x + 1` | Functions are values |

Check a value's runtime type with `type(...)`:

```xe
print(type(42))
print(type("XE"))
print(type([1, 2, 3]))
print(type({"a": 1}))
print(type(none))
```

## Variables

```xe
score = 10
score = score + 5
score += 5
print(score)
```

A variable's type is fixed by its first assignment: after `score = 10`, assigning text to `score` is a compile error. There is no `const` keyword yet.

## Numbers

Numbers support `+`, `-`, `*`, `/`, `//` (divide and round down), `%` and `**` (power):

```xe
print(7 / 2, 7 // 2, 7 % 3, 2 ** 10) # 3.5 3 1 1024
print(-7 // 2, -7 % 3)               # -4 2
```

Division and modulo by zero raise runtime errors instead of silently returning fallback values.

Math functions: `abs`, `floor`, `ceil`, `sqrt`, `round(x)`, `round(x, digits)`, `min`, `max`, `sum`, `random()`.

## Text

Text values use double quotes. Escapes: `\n`, `\t`, `\r`, `\\`, `\"`.

```xe
name = "XE"
print("Hello " + name)
print("Count: " + 3) # + converts the other value to text
```

Text can be indexed and sliced by character, and compared alphabetically:

```xe
word = "héllo"
print(length(word), word[1], word[-1], word[1:3]) # 5 é o él
print("apple" < "banana")                        # true
```

Text functions: `upper`, `lower`, `trim`, `replace(text, old, new)`, `split(text)`, `split(text, separator)`, `join(list, separator)`, `starts_with`, `ends_with`, `find(text, part)`. Text cannot be changed in place; these return new text.

## Booleans and truthiness

Any value can be used as a condition. `false`, `none`, `0`, `""`, `[]` and `{}` are false; everything else is true.

```xe
if [1, 2]:
    print("non-empty lists are truthy")
```

## none

`none` means "no value". Functions without a `return` value return it:

```xe
fun log(message):
    print(message)

result = log("hi")
print(result == none) # true
```

## Lists

```xe
items = [10, 20, 30]
print(items[0], items[-1]) # 10 30
print(items[1:])           # [20, 30]
print(length(items))

items[1] = 99
items.append(40)           # same as append(items, 40)
last = items.pop()         # 40
items.insert(0, 5)
print(items)               # [5, 10, 99, 30]
print(99 in items)         # true
print(items.sort())        # sorts in place: [5, 10, 30, 99]
```

Other list functions: `remove(list, value)`, `reverse(list)`, `copy(list)`, `sum(list)`, `min(list)`, `max(list)`, `range(...)`.

### Lists are shared

Assigning a list or passing it to a function does not copy it, as in Python:

```xe
fun add_item(list):
    list.append("new")

a = []
b = a
add_item(b)
print(a) # ["new"]
```

Use `copy(list)` when you need an independent copy. The same applies to maps and structs.

## Maps

Map keys can be numbers, text or booleans; `1` and `"1"` are different keys. Maps keep insertion order.

```xe
user = {
    "name": "Alice",
    "age": 30
}

print(user["name"], user.name) # text keys can also be read like fields
user["age"] += 1
user["admin"] = true

print(has_key(user, "age"), "age" in user)
print(keys(user))   # ["name", "age", "admin"]
remove(user, "admin")

for key in user:
    print(key, user[key])
```

## Structs

```xe
struct Point:
    x
    y

p = Point(10, 20)
print(p.x, p.y) # 10 20
p.x = 42
print(p)        # Point { x: 42, y: 20 }

print(Point(1, 2) == Point(1, 2)) # true: structs compare by value
```

Assigning a field that the struct does not declare is an error.

## Conversion rules

`convert(value, target)` supports three targets:

```xe
print(convert("42", "number"))
print(convert(123, "text"))
print(convert("", "boolean"))
```

- text to number must parse successfully; otherwise it is a runtime error
- booleans convert to `1` or `0`
- `"boolean"` uses truthiness

## Equality and comparison

- `==` and `!=` compare values deeply. Values of different kinds are never equal: `true == 1` is `false`.
- `<`, `>`, `<=` and `>=` compare two numbers or two texts. Comparing a number with text is an error.

## Printing

`print` shows text as-is, but inside lists and maps text is quoted so that `1` and `"1"` look different:

```xe
print("a", ["a", 1], {"k": "v"}) # a ["a", 1] {"k": "v"}
```

## Next steps

- Continue with [Control Flow](/guide/control-flow)
- Or jump to [Functions and Scope](/guide/functions-and-scope)
