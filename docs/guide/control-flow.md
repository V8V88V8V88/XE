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

- conditions use truthiness
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

XE currently allows `for` loops over:

- lists
- text values

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

## Scope inside control-flow blocks

XE does not automatically export names created inside `if` or loop blocks.

```xe
if true:
    inner = 42

print(inner)
```

That fails because `inner` was created inside the block.

But reassignment to an already existing outer variable works:

```xe
count = 0

repeat 3 times:
    count = count + 1

print(count)
```

## Short-circuit behavior

Logical `and` and `or` use short-circuit evaluation:

- `left and right` only evaluates `right` if `left` is truthy
- `left or right` only evaluates `right` if `left` is falsey

This matters when the right-hand side contains function calls or conversions that may fail.

## Next steps

- Continue with [Functions and Scope](/guide/functions-and-scope)
- Then read [Runtime Behavior and Errors](/guide/runtime-and-errors)
