# XE for Visual Studio Code

Editor support for the [XE programming language](https://xe-lang.vercel.app): an indentation-based language that compiles to native binaries through Rust.

## Features

- Syntax highlighting for keywords, built-in functions, strings, numbers, operators, function and struct definitions
- Comment toggling with `#`
- Auto-indent after `:` and outdent after `return`, `break`, `continue` and `pass`
- Indentation-based code folding
- Snippets: `fun`, `struct`, `if`, `ifelse`, `for`, `forr`, `while`, `repeat`, `try`, `main`

```xe
struct Point:
    x
    y

fun distance(a, b):
    return sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2)

print("Distance:", distance(Point(0, 0), Point(3, 4)))
```

## Getting XE

```bash
curl -fsSL https://xe-lang.vercel.app/install.sh | bash
xe run hello.xe
```

See the [documentation](https://xe-lang.vercel.app/docs) to learn the language.

## Issues

Report problems at [github.com/V8V88V8V88/XE/issues](https://github.com/V8V88V8V88/XE/issues).
