# Keyword Reference

Keywords are reserved words that have a special meaning in XE. They cannot be used as variable names or function names.

## All Reserved Keywords

| Keyword | Category | Purpose |
| --- | --- | --- |
| `fun` | Declarations | Define a new function |
| `function` | Declarations | Alias for `fun` (legacy support) |
| `if` | Control Flow | Start a conditional block |
| `elif` | Control Flow | Add a conditional branch to an `if` statement |
| `else` | Control Flow | Fallback branch for an `if` statement |
| `while` | Loops | Start a conditional loop |
| `for` | Loops | Iterate over a list or text |
| `in` | Loops | Used in the `for` loop syntax |
| `repeat` | Loops | Start a fixed-count loop |
| `times` | Loops | Used in the `repeat` loop syntax |
| `break` | Loops | Exit the current loop immediately |
| `continue` | Loops | Skip to the next iteration of the loop |
| `and` | Logic | Logical AND operator |
| `or` | Logic | Logical OR operator |
| `not` | Logic | Logical NOT operator |
| `true` | Literals | Boolean true value |
| `false` | Literals | Boolean false value |
| `return` | Functions | Return a value from a function |
| `import` | Modules | Import an entire module |
| `from` | Modules | Import specific names from a module |

## Details by Category

### Declarations

- **`fun`**: The primary keyword for defining functions. Example: `fun add(a, b):`.
- **`function`**: A legacy alias for `fun`.

### Control Flow

- **`if`**, **`elif`**, **`else`**: Used to build decision logic. XE requires a colon `:` after the condition and an indented block for the body.

### Loops

- **`while`**: Repeats a block as long as a condition is true.
- **`for ... in ...`**: Iterates through every element in a list or every character in a text value.
- **`repeat ... times`**: A high-level loop for repeating an action a specific number of times.
- **`break`**: Stops the execution of the innermost loop.
- **`continue`**: Skips the current iteration and goes to the next check/value in the loop.

### Logical Operators

- **`and`**: Returns true if both operands are true.
- **`or`**: Returns true if at least one operand is true.
- **`not`**: Inverts a boolean value.

### Module System

- **`import`**: Used to load another `.xe` file as a module.
- **`from`**: Used to pull specific functions out of a module into the current namespace.

## Reserved Word Rule

If you try to use a keyword as a name, XE will report a syntax error during the parsing phase.

```xe
# This will fail
if = 10 
```

Error: `expected expression at line 2, column 4`
