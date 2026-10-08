# Language Basics

XE is intentionally small. The syntax is indentation-based and close to Python in shape, but the compiler lowers everything into Rust.

If you want the book-like path through the docs, read the chapters in this order:

1. [Syntax and Blocks](/guide/syntax-and-blocks)
2. [Types and Values](/guide/types-and-values)
3. [Control Flow](/guide/control-flow)
4. [Functions and Scope](/guide/functions-and-scope)
5. [Runtime Behavior and Errors](/guide/runtime-and-errors)

## Hello World

Every language starts with printing a message.

```xe
print("Hello, World!")
```

`print` is a built-in function. It writes values to the screen.

## Numbers and variables

Variables are just names that hold values.

```xe
x = 10
y = 20
print(x + y)
```

What happens:

- `x` gets `10`
- `y` gets `20`
- `x + y` evaluates to `30`
- `print` shows the result

Arithmetic operators:

- `+`, `-`, `*`, `/`
- `//` divides and rounds down, `%` is the remainder, `**` is power
- `x += 1` is short for `x = x + 1` (also `-=`, `*=`, `/=`, `//=`, `%=`, `**=`)

## Text

Text values are written in double quotes.

```xe
name = "XE"
print("Hello " + name)
```

The `+` operator can also join text values.

## Booleans

Boolean values are `true` and `false`.

```xe
ready = true
print(type(ready))
```

## Input and conversion

`input()` always returns text, even when the user types digits. Convert it before using it as a number.

```xe
age_text = input("Enter your age: ")
age = convert(age_text, "number")

if age >= 18:
    print("Adult")
else:
    print("Minor")
```

Valid conversion targets currently documented in XE:

- `"number"`
- `"text"`
- `"boolean"`

## Conditionals

XE uses indentation-based blocks for `if`, `elif`, and `else`.

```xe
score = 82

if score >= 90:
    print("A")
elif score >= 80:
    print("B")
else:
    print("Keep going")
```

Comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=` (numbers, or text alphabetically), and `in` / `not in` for membership.

Logical operators: `and`, `or`, `not`. As in Python, `not x == 5` means `not (x == 5)`.

Conditions can be any value: `0`, `""`, `[]`, `{}`, `none` and `false` count as false.

Variables assigned inside an `if` block are still available after it.

## Repeat loops

XE currently has fixed-count loops through `repeat N times`.

```xe
repeat 3 times:
    print("XE")
```

`N` can be a number or a variable that stores a number.

Assignments inside the loop update outer variables if they already exist:

```xe
count = 0

repeat 3 times:
    count = count + 1

print(count)
```

## While loops

Use `while` when the loop should keep running until a condition becomes false.

```xe
count = 0

while count < 3:
    print(count)
    count = count + 1
```

## For loops

Use `for name in iterable` to walk through a list, a text value, the keys of a map, or a `range` of numbers.

```xe
for i in range(3):
    print(i) # 0, 1, 2
```

```xe
total = 0

for item in [1, 2, 3]:
    total = total + item

print(total)
```

For text, XE iterates one character at a time:

```xe
for ch in "XE":
    print(ch)
```

## Break and continue

Inside `repeat`, `while`, and `for` loops you can use:

- `break` to leave the loop immediately
- `continue` to skip to the next iteration

```xe
count = 0

while true:
    count = count + 1
    if count == 2:
        continue
    if count == 4:
        break
    print(count)
```

## Functions

Functions let you name reusable logic.

```xe
fun add(a, b):
    return a + b

print(add(3, 5))
```

A function that does not return a value gives back `none`:

```xe
fun greet(name):
    print("Hello " + name)

greet("World")
```

Functions are values too: pass them to other functions, or write small ones inline with `lambda x: x * 2`.

## Modules

XE can import from other `.xe` files.

```xe
from math_utils import double
print(double(21))
```

Or import the whole module and use its name:

```xe
import helpers
print(helpers.square(4))
```

Imports resolve relative to the current file and must come before executable top-level statements.

## Recursion

Recursion works and is useful for classic examples like Fibonacci.

```xe
fun fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print(fib(10))
```

## Lists

Lists hold ordered values.

```xe
fruits = ["apple", "banana", "cherry"]
print(fruits[0])
print(length(fruits))
```

Notes:

- list indexing starts at `0`; `fruits[-1]` is the last element
- `fruits[1:]` is a slice: a new list without the first element
- `fruits.append("date")` adds to the end (same as `append(fruits, "date")`)
- `length(fruits)` returns the number of elements

## Built-ins

XE has built-in functions for text (`upper`, `split`, `replace`, ...), lists (`append`, `pop`, `sort`, ...), maps (`keys`, `values`, ...), math (`abs`, `round`, `sqrt`, `min`, `max`, ...), files (`read_file`, `write_file`, ...) and the program (`args`, `exit`, `error`). See the [Language Reference](/reference/language#built-in-functions) for the full list.

## Quick summary

- Data types: number, text, boolean, none, list, map, struct, function
- Typing: inferred; known numbers, text and booleans compile to native values
- Control flow: `if`, `elif`, `else`, `repeat N times`, `while`, `for ... in ...`, `try` / `catch`
- Loop control: `break`, `continue`, `pass`
- Functions: `fun name(args):`, `return`, and `lambda`
- Blocks: indentation with spaces, not braces

For the more detailed version of each topic, continue with:

- [Syntax and Blocks](/guide/syntax-and-blocks)
- [Types and Values](/guide/types-and-values)
- [Control Flow](/guide/control-flow)
- [Functions and Scope](/guide/functions-and-scope)
- [Runtime Behavior and Errors](/guide/runtime-and-errors)
