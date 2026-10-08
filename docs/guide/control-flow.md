# Control Flow

Control flow is where XE starts to feel like a small usable language instead of only a syntax demo. This chapter covers branching, looping, and loop control.

## `if`, `elif`, and `else`

XE conditionals look like this:

```xe
score = 82

if score >= 90:
    print("A")
elif score >= 80:
    print("B")
else:
    print("Keep going")
```

Rules:

- conditions use truthiness: `false`, `none`, `0`, `""`, `[]` and `{}` are false, everything else is true
- each branch is an indented block
- `elif` chains are supported
- `else` is optional

## `repeat N times`

Use `repeat` for a fixed-count loop:

```xe
repeat 3 times:
    print("XE")
```

The count must be a non-negative integer at runtime.

These fail:

- negative counts
- decimal counts such as `2.5`
- non-number counts

## `while`

Use `while` when the number of iterations depends on a condition:

```xe
count = 0

while count < 3:
    print(count)
    count = count + 1
```

The condition is checked before each iteration.

## `for ... in ...`

XE allows `for` loops over:

- lists
- text values, one character at a time
- maps, which visit their keys in insertion order
- `range(...)`, which counts numbers

Count with `range`:

```xe
for i in range(5):
    print(i) # 0 to 4

for i in range(10, 0, -2):
    print(i) # 10, 8, 6, 4, 2
```

Example over a list:

```xe
total = 0

for item in [1, 2, 3, 4]:
    total = total + item

print(total)
```

Example over text:

```xe
for ch in "XE":
    print(ch)
```

For text, each iteration variable receives a one-character text value.

## `break`

`break` exits the nearest loop immediately.

```xe
count = 0

while true:
    count = count + 1
    if count == 3:
        break

print(count)
```

`break` outside a loop is a compiler error.

## `continue`

`continue` skips the rest of the current iteration and starts the next one.

```xe
count = 0
total = 0

while count < 5:
    count = count + 1
    if count == 3:
        continue
    total = total + count

print(total)
```

`continue` outside a loop is also a compiler error.

## `pass`

A block cannot be empty; write `pass` where there is nothing to do:

```xe
for item in [1, 2, 3]:
    pass
```

## `try` and `catch`

`try` runs a block, and if a runtime error happens inside it, runs the `catch` block instead of stopping the program. The name after `catch` (optional) receives the error message:

```xe
fun safe_divide(a, b):
    try:
        return a / b
    catch err:
        print("could not divide:", err)
        return 0

print(safe_divide(1, 0))
```

Raise your own errors with `error("message")`.

## Scope inside control-flow blocks

Blocks do not create a new scope, as in Python. A name assigned inside an `if`, loop or `try` block is visible after it:

```xe
score = 70
if score >= 50:
    result = "pass"
else:
    result = "fail"
print(result)
```

Inside a function, a variable must be assigned on every path before it is read; otherwise compiling fails with "might not be assigned yet". A `for` loop's own variable belongs to the loop.

## Short-circuit behavior

Logical `and` and `or` use short-circuit evaluation:

- `left and right` only evaluates `right` if `left` is truthy
- `left or right` only evaluates `right` if `left` is falsey

This matters when the right-hand side contains function calls or conversions that may fail.

## Next steps

- Continue with [Functions and Scope](/guide/functions-and-scope)
- Then read [Runtime Behavior and Errors](/guide/runtime-and-errors)
