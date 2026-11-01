# Editor Support

## Visual Studio Code

The official **XE Lang** extension adds language support for `.xe` files.

[Install from the Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=xe-lang.xe-language)

Or install it from the command line:

```bash
code --install-extension xe-lang.xe-language
```

### Features

- Syntax highlighting for keywords, built-in functions, strings, numbers, operators, and function and struct definitions
- Comment toggling with `#`
- Auto-indent after `:` and outdent after `return`, `break`, `continue` and `pass`
- Indentation-based code folding
- Snippets for common blocks

### Snippets

Type a prefix and press <kbd>Tab</kbd>:

| Prefix | Inserts |
| :--- | :--- |
| `fun` | Function definition |
| `struct` | Struct definition |
| `if` | If statement |
| `ifelse` | If / else statement |
| `for` | Loop over a list, text or map |
| `forr` | Loop over a range of numbers |
| `while` | While loop |
| `repeat` | Repeat a block a fixed number of times |
| `try` | Handle errors with try / catch |
| `main` | Main function with a call to it |

The extension's source lives in [`editors/vscode`](https://github.com/V8V88V8V88/XE/tree/main/editors/vscode) in the XE repository.

## Other editors

There is no dedicated support for other editors yet. Because XE uses Python-style indentation, setting `.xe` files to use Python highlighting is a reasonable stopgap.

Contributions for other editors are welcome. See [Contributing](./contributing).
