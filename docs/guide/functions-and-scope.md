# Functions and Scope

Functions are one of the most important parts of XE because they show how the language handles parameters, returns, recursion, and name lookup.

## Defining a function

Use the `fun` keyword:

```xe
fun add(a, b):
    return a + b
```

Call it like this:

```xe
print(add(3, 5))
```

## Parameters

Parameters are local names inside the function body.

```xe
fun greet(name):
    print("Hello " + name)
    return 0
```

XE checks argument count during semantic analysis, so calling a function with the wrong number of arguments is a compiler error.

## Return values

Use `return` to send a value back to the caller.

```xe
fun square(n):
    return n * n
```

Current rules:

- `return` is only valid inside functions
- if a function reaches the end without returning a value, or uses a bare `return`, the call evaluates to `none`

```xe
fun greet():
    print("hi")

x = greet()
print(x) # none
```

## Scope model

Functions can access:

- their parameters
- values created inside the function body
- built-in functions
- other user-defined functions
- imported user-defined functions
- **global module-level variables**

Example of global access:

```xe
x = 10

fun show():
    print(x)

show()
```

Functions read the current value of a global, including changes made by top-level loops and by other functions. Lists, maps and structs are shared, so mutating one (with `append`, `items[0] = ...`, `p.x = ...`) changes it everywhere it is used, whether it is a global or was passed in as an argument.

```xe
items = []

fun add(x):
    append(items, x)

add(1)
print(items) # [1]
```

## Local variables and `global`

XE follows Python's rule: a name that is **assigned** anywhere inside a function is local to that whole function.

- the first assignment creates it, even inside an `if`, `while` or `try` block
- later assignments reuse that same variable
- a local may have the same name as a global; the global is left untouched

```xe
fun describe(n):
    if n > 0:
        label = "positive"
    else:
        label = "not positive"
    return label
```

A local must be assigned on every path before it is read. This is checked when compiling:

```xe
fun first_even(items):
    for item in items:
        if item % 2 == 0:
            found = item
    return found # Error: variable 'found' might not be assigned yet
```

Give the variable a starting value (`found = none`) before the loop to fix it.

```xe
fun counter():
    total = 0
    total = total + 1
    return total
```

To assign to a module-level variable from inside a function, declare it with `global`:

```xe
count = 0

fun increment():
    global count
    count = count + 1

increment()
print(count) # 1
```

Reading a name before assigning it in the same function is a compile error, because the name is local there:

```xe
count = 0

fun increment():
    count = count + 1 # Error: local variable 'count' is used before it is assigned
```

`global` can also create a module-level variable from inside a function. Using a global before any assignment to it has run is a runtime error.

## Recursion

Functions can call themselves:

```xe
fun fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
```

Functions can be called before they are defined in the file.

## Functions are values

A function can be stored in a variable, passed to another function, or returned:

```xe
fun double(x):
    return x * 2

fun apply(f, items):
    out = []
    for item in items:
        out.append(f(item))
    return out

print(apply(double, [1, 2, 3])) # [2, 4, 6]
print(apply(upper, ["a", "b"])) # built-in functions work too
```

## Lambdas

`lambda params: expression` creates a small anonymous function:

```xe
print(apply(lambda x: x + 100, [1, 2])) # [101, 102]

fun make_adder(n):
    return lambda x: x + n

add5 = make_adder(5)
print(add5(10)) # 15
```

A lambda can use variables from around it. Their values are copied when the lambda is created, so later reassignments do not affect it (lists and maps are still shared).

Functions and structs can only be defined at the top level of a module; use a lambda when you need a function inside another one.

## Method-call syntax

`value.name(args)` is another way to write `name(value, args)`:

```xe
items = [3, 1, 2]
items.append(0)
print(items.sort(), "hi".upper())
```

## No contracts or constants yet

Two features that often come up in language discussions are not in XE right now:

- there is no function contract syntax such as `requires` or `ensures`
- there is no `const` binding syntax; all bindings are currently mutable

## Next steps

- Continue with [Runtime Behavior and Errors](/guide/runtime-and-errors)
- Or inspect the runnable [Examples](/guide/examples)
