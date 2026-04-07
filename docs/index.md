---
layout: home

hero:
  name: XE
  text: Docs for a small programming language with a Rust backend
  tagline: Write XE code, compile into Rust, and run native binaries while keeping the language easy to read.
  image:
    src: /XElogo.png
    alt: XE logo
  actions:
    - theme: brand
      text: Get Started
      link: /guide/getting-started
    - theme: alt
      text: Learn the Language
      link: /guide/language-basics
    - theme: alt
      text: See Examples
      link: /guide/examples

features:
  - title: Small, readable syntax
    details: XE uses indentation-based blocks, simple assignment, functions, lists, and built-ins designed to feel approachable.
  - title: Rust-powered output
    details: XE compiles to Rust source and then uses rustc to produce the final executable.
  - title: Built for learning
    details: The compiler pipeline is compact enough to study directly from the repository.
  - title: Standard docs setup
    details: This site is built with VitePress so the docs look and behave like a modern developer docs site.
---

## What XE is

XE is a source-to-source programming language. The flow is:

1. Write a `.xe` file.
2. XE lexes, parses, and validates it.
3. XE generates Rust code.
4. `rustc` builds the native executable.

This project is pre-alpha and intended for learning and experimentation, not production use.

## What you can do today

- Work with numbers, text, booleans, and lists
- Use `if`, `elif`, and `else`
- Run `repeat`, `while`, and `for ... in ...` loops
- Use `break` and `continue` inside loops
- Define functions and use recursion
- Use built-ins like `print`, `input`, `length`, `type`, and `convert`
- Reassign outer variables from inside nested blocks without accidental shadowing

## How to learn it

The docs are now organized in one path instead of a split between "new docs" and an older book:

1. Start with [Getting Started](/guide/getting-started) to build and run XE.
2. Read [Language Basics](/guide/language-basics) for syntax and explanations.
3. Open [Examples](/guide/examples) for runnable programs from the repository.
4. Use [Reference](/reference/language) when you need quick lookup pages.

## Next stops

- [Getting Started](/guide/getting-started)
- [Language Basics](/guide/language-basics)
- [Examples](/guide/examples)
- [CLI Reference](/reference/cli)
