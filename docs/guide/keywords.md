# Keyword Reference

Keywords are reserved words that have a special meaning in XE. They cannot be used as variable names or function names.

## All Reserved Keywords

| Keyword | Category | Purpose |
| --- | --- | --- |
| `fun` | Declarations | Define a new function |
| `fn` | Declarations | Short alias for `fun` |
| `function` | Declarations | Alias for `fun` (legacy support) |
| `struct` | Declarations | Define a new composite data structure |
| `if` | Control Flow | Start a conditional block |
| `elif` | Control Flow | Add a conditional branch to an `if` statement |
| `else` | Control Flow | Fallback branch for an `if` statement |
| `while` | Loops | Start a conditional loop |
| `for` | Loops | Iterate over a list, text, or map |
| `in` | Loops | Used in the `for` loop syntax |
| `repeat` | Loops | Start a fixed-count loop |
| `times` | Loops | Used in the `repeat` loop syntax |
| `break` | Loops | Exit the current loop immediately |
| `continue` | Loops | Skip to the next iteration of the loop |
| `pass` | Control Flow | A statement that does nothing, for empty blocks |
| `try` | Errors | Run a block and handle runtime errors in it |
| `catch` | Errors | The block that runs when a `try` block fails |
| `and` | Logic | Logical AND operator |
| `or` | Logic | Logical OR operator |
| `not` | Logic | Logical NOT operator (also part of `not in`) |
| `true` | Literals | Boolean true value |
| `false` | Literals | Boolean false value |
| `none` | Literals | The absence of a value |
| `return` | Functions | Return a value from a function |
| `global` | Functions | Let a function assign to a module-level variable |
| `lambda` | Functions | Create an anonymous function: `lambda x: x * 2` |
| `import` | Modules | Import an entire module |
| `from` | Modules | Import specific names from a module |
| `as` | Modules | Rename an import: `import helpers as h` |

## Details by Category

### Declarations

- **`fun`**, **`fn`**: Define functions. Example: `fun add(a, b):` or `fn add(a, b):`.
- **`function`**: A legacy alias for `fun`.
- **`struct`**: Defines user-defined composite data types with named fields. Example:
  ```xe
  struct Point:
      x
      y
  ```
- **`global`**: Inside a function, marks names as module-level variables so assignments update them instead of creating locals. Example: `global count`. See [Functions and Scope](/guide/functions-and-scope).

### Control Flow

- **`if`**, **`elif`**, **`else`**: Used to build decision logic. XE requires a colon `:` after the condition and an indented block for the body.
- **`pass`**: Does nothing; use it where a block is required but there is nothing to do.
- **`try`**, **`catch`**: Handle runtime errors. Example:
  ```xe
  try:
      n = convert("abc", "number")
  catch err:
      print("failed:", err)
  ```

### Loops

- **`while`**: Repeats a block as long as a condition is true.
- **`for ... in ...`**: Iterates through every element in a list, every character in a text value, or every key in a map.
- **`repeat ... times`**: A high-level loop for repeating an action a specific number of times.
- **`break`**: Stops the execution of the innermost loop.
- **`continue`**: Skips the current iteration and goes to the next check/value in the loop.

### Logical Operators

- **`and`**: Returns true if both operands are true.
- **`or`**: Returns true if at least one operand is true.
- **`not`**: Inverts the truthiness of a value. `x not in items` checks that `x` is absent.

### Module System

- **`import`**: Loads another `.xe` file as a module; its members are used as `module.name`.
- **`from`**: Pulls specific functions, structs or variables out of a module into the current namespace.
- **`as`**: Gives an imported module or name a different local name.

## Reserved Word Rule

If you try to use a keyword as a name, XE will report a syntax error during the parsing phase.

```xe
# This will fail
if = 10 
```

Error: `expected expression at line 2, column 4`
