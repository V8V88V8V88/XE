# Examples

These examples map directly to files in the repository's `examples/` directory.

## Hello World

Source: `examples/hello.xe`

```xe
print("Hello, World!")
```

## Functions and repeat

Source: `examples/functions.xe`

```xe
function greet(name):
    print("Hello " + name)
    return 0

greet("World")

repeat 3 times:
    print("XE is cool!")
```

## While loop

Source: `examples/while_loop.xe`

```xe
count = 0

while count < 5:
    print(count)
    count = count + 1
```

## For loop

Source: `examples/for_loop.xe`

```xe
total = 0

for item in [1, 2, 3, 4]:
    total = total + item

print(total)
```

## Elif chain

Source: `examples/elif.xe`

```xe
score = 82

if score >= 90:
    print("A")
elif score >= 80:
    print("B")
else:
    print("Keep going")
```

## Lists

Source: `examples/lists.xe`

```xe
fruits = ["apple", "banana", "cherry"]
print("First fruit: " + fruits[0])
print("Second fruit: " + fruits[1])
print("List length: " + convert(length(fruits), "text"))
```

## Input demo

Source: `examples/input_demo.xe`

```xe
name = input("Enter your name: ")
print("Hello, " + name)

age_text = input("Enter your age: ")
age = convert(age_text, "number")
next_year = age + 1
print("Next year you will be: " + convert(next_year, "text"))
```

## Built-ins

Source: `examples/builtins.xe`

```xe
print("length of 'hello':", length("hello"))
print("type of 42:", type(42))
x = convert("100", "number")
print("converted:", x + 5)
```

## Fibonacci

Source: `examples/fib.xe`

```xe
function fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print(fib(10))
```

## More practice examples

### Factorial

```xe
function factorial(n):
    if n <= 0:
        return 1
    return n * factorial(n - 1)

print(factorial(5))
```

### Nested if

```xe
age = 20
has_ticket = true

if age >= 18:
    if has_ticket:
        print("You can enter")
    else:
        print("Need a ticket")
else:
    print("Must be 18 or older")
```

### Sum 1 to N

```xe
function sum_to_n(n):
    if n <= 0:
        return 0
    return n + sum_to_n(n - 1)

print(sum_to_n(10))
```
