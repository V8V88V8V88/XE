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
- if a function reaches the end without an explicit `return`, XE returns `0`

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

That is valid in XE because global variables are stored in a registry that all functions can see.

## Local variables vs reassignment

Inside a function, a name behaves like any other XE name:

- first assignment creates it in the current local scope
- later assignment reuses that same variable

```xe
fun counter():
    total = 0
    total = total + 1
    return total
```

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
