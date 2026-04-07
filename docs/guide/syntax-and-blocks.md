# Syntax and Blocks

This chapter explains the surface structure of XE programs: statements, blocks, indentation, and a few rules that make code parse correctly.

## Statements end at the newline

XE is line-oriented. In most cases, one statement lives on one line.

```xe
x = 10
print(x)
```

The parser treats the newline as the end of the statement unless it is still inside parentheses or brackets.

## Blocks use indentation

XE does not use braces. A block starts after a block header and is defined by indentation.

```xe
if true:
    print("inside the block")

print("outside the block")
```

The indented line belongs to the `if` block. The final line does not.

## Why XE still uses `:`

Indentation tells XE where the block starts and ends.

The colon tells XE that the block header is complete and a block is about to begin.

```xe
while count < 5:
    count = count + 1
```

This makes block-introducing statements easier to read and simpler to parse consistently.

## Spaces vs tabs

XE expects indentation to be consistent. The lexer accepts spaces and also counts a tab as four spaces, but mixed or inconsistent indentation can raise an `invalid indentation` compiler error.

Best practice:

- use spaces
- keep one indentation width everywhere in the file
- do not mix indentation styles inside the same block structure

## Block-introducing forms

These statements start a block:

- `if condition:`
- `elif condition:`
- `else:`
- `repeat count times:`
- `while condition:`
- `for name in iterable:`
- `function name(args):`

## Comments

XE uses `#` for single-line comments.

```xe
# This line is ignored
count = 3  # this trailing text is also ignored
```

There is no multi-line comment syntax right now.

## Parentheses and brackets

Parentheses are used for function calls and grouping:

```xe
print((10 + 20) * 2)
```

Brackets are used for lists and indexing:

```xe
items = [10, 20, 30]
print(items[1])
```

## What XE does not support in syntax yet

These are not part of the language right now:

- `const` declarations
- `fun` as a function keyword
- list mutation like `items[0] = 42`
- modules or `import`
- multi-line strings

## Next steps

- Continue with [Types and Values](/guide/types-and-values)
- Then read [Control Flow](/guide/control-flow)
