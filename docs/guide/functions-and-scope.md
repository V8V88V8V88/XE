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

Functions read the current value of a global, including changes made by top-level loops and by other functions. Mutating a global list, map, or struct (with `append`, `pop`, `items[0] = ...`, `p.x = ...`) changes the global itself.

```xe
items = []

fun add(x):
    append(items, x)

add(1)
print(items) # [1]
```

## Local variables and `global`

XE follows Python's rule: a name that is **assigned** anywhere inside a function is local to that function.

- first assignment creates it in the current local scope
- later assignment reuses that same variable
- a local may have the same name as a global; the global is left untouched

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

XE supports recursion naturally because function definitions are collected before semantic checks.

## No nested closures yet

XE does not currently support **nested closures** (capturing a local variable from an outer function).

```xe
fun outer():
    x = 10
    fun inner():
        print(x) # Error: inner() cannot see x from outer()
```

Imports do not change that rule. A function can call an imported XE function and read global variables, but it cannot capture local variables from another function's stack frame.

## No contracts or constants yet

Two features that often come up in language discussions are not in XE right now:

- there is no function contract syntax such as `requires` or `ensures`
- there is no `const` binding syntax; all bindings are currently mutable

## Next steps

- Continue with [Runtime Behavior and Errors](/guide/runtime-and-errors)
- Or inspect the runnable [Examples](/guide/examples)
