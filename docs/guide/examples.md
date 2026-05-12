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
fun greet(name):
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
fun fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

print(fib(10))
```

## Modules

Source folder: `examples/modules/`

Files:

- `main.xe` (entry)
- `math_utils.xe`
- `strings.xe`

`main.xe`
```xe
from math_utils import double
from strings import shout

result = double(21)
print(shout(convert(result, "text")))
```

`math_utils.xe`
```xe
fun double(n):
    return n * 2
```

`strings.xe`
```xe
fun shout(value):
    return value + "!"
```

Run it:

```bash
xe run examples/modules/main.xe
```

## More practice examples

### Factorial

```xe
fun factorial(n):
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
fun sum_to_n(n):
    if n <= 0:
        return 0
    return n + sum_to_n(n - 1)

print(sum_to_n(10))
```

## Advanced & Practical Examples

### To-Do List Manager

Source: `examples/todo.xe`

A simple interactive task manager demonstrating list operations, loops, and user input.

```xe
tasks = []
print("--- XE To-Do List Manager ---")

running = true
while running:
    print("")
    print("You have " + convert(length(tasks), "text") + " task(s):")
    
    if length(tasks) == 0:
        print("(None)")
    else:
        for t in tasks:
            print("- " + t)
    
    print("")
    print("1. Add task")
    print("2. Clear all")
    print("3. Exit")
    
    choice = input("Choose an option: ")
    
    if choice == "1":
        new_task = input("What needs to be done? ")
        tasks = tasks + [new_task]
        print("Task added!")
    elif choice == "2":
        tasks = []
        print("List cleared.")
    elif choice == "3":
        running = false
    else:
        print("Invalid choice, try again.")
```

### Dark Forest Adventure

Source: `examples/adventure.xe`

A choice-based mini-game showing how to use flags and nested conditionals for game logic.

```xe
print("Welcome to the Dark Forest!")
print("You are a traveler searching for the Lost Gem.")

playing = true
has_coin = false

while playing:
    print("")
    print("You are at a clearing. What do you do?")
    print("1. Go North into the dense woods")
    print("2. Go South towards the whispering river")
    print("3. Check your pockets")
    print("4. Give up and go home")
    
    choice = input("> ")
    
    if choice == "1":
        print("A giant spider blocks your path!")
        if has_coin:
            print("You throw your shiny coin at it. The spider is confused.")
            print("You run past it and find the Lost Gem! YOU WIN!")
            playing = false
        else:
            print("You have nothing to defend yourself with. You retreat.")
            
    elif choice == "2":
        if has_coin:
            print("The river is beautiful, but you've already found what it hides.")
        else:
            print("You find a shiny gold coin at the riverbank!")
            has_coin = true
            
    elif choice == "3":
        if has_coin:
            print("You have a shiny gold coin.")
        else:
            print("Your pockets are empty.")
            
    elif choice == "4":
        print("The forest wins this time. Goodbye!")
        playing = false
        
    else:
        print("You wander in circles...")
```

### Unit Converter

Source: `examples/converter.xe`

A practical tool for temperature and distance conversions.

```xe
print("--- XE Unit Converter ---")

running = true
while running:
    print("")
    print("1. Celsius to Fahrenheit")
    print("2. Kilometers to Miles")
    print("3. Quit")
    
    choice = input("Select a conversion: ")
    
    if choice == "1":
        c_text = input("Enter temperature in Celsius: ")
        c = convert(c_text, "number")
        f = (c * 9 / 5) + 32
        print(c_text + " C is " + convert(f, "text") + " F")
        
    elif choice == "2":
        km_text = input("Enter distance in Kilometers: ")
        km = convert(km_text, "number")
        mi = km * 0.621371
        print(km_text + " km is approximately " + convert(mi, "text") + " miles")
        
    elif choice == "3":
        running = false
        
    else:
        print("Invalid option.")
```

### Palindrome Checker

Source: `examples/palindrome.xe`

Checks if a word reads the same forwards and backwards, demonstrating string indexing and reversal logic.

```xe
print("--- Palindrome Checker ---")
word = input("Enter a word: ")

reversed_word = ""
length_word = length(word)

i = length_word - 1
while i >= 0:
    reversed_word = reversed_word + word[i]
    i = i - 1

print("Reversed: " + reversed_word)

if word == reversed_word:
    print("Yes, '" + word + "' is a palindrome!")
else:
    print("No, '" + word + "' is not a palindrome.")
```

### Student Grade System

Source: `examples/grades.xe`

Collects grades for multiple subjects, calculates the average, and determines pass/fail status.

```xe
print("--- XE Grade System ---")

student_name = input("Student Name: ")
num_subjects = convert(input("Number of subjects: "), "number")

total = 0
i = 0
while i < num_subjects:
    prompt = "Grade for subject " + convert(i + 1, "text") + ": "
    grade = convert(input(prompt), "number")
    total = total + grade
    i = i + 1

if num_subjects > 0:
    average = total / num_subjects
    print("Student: " + student_name)
    print("Average: " + convert(average, "text"))
    if average >= 50:
        print("Status: PASSED")
    else:
        print("Status: FAILED")
```

### Prime Numbers

Source: `examples/prime_numbers.xe`

Finds all prime numbers up to a user-specified limit using nested loops.

```xe
limit = convert(input("Enter a limit: "), "number")
n = 2
while n <= limit:
    is_prime = true
    d = 2
    while d * d <= n:
        if n % d == 0:
            is_prime = false
        d = d + 1
    if is_prime:
        print(n)
    n = n + 1
```

### Multiplication Table

Source: `examples/multiplication_table.xe`

Generates a multiplication grid of a given size.

```xe
size = convert(input("Enter table size: "), "number")
y = 1
while y <= size:
    row = ""
    x = 1
    while x <= size:
        row = row + convert(x * y, "text") + " "
        x = x + 1
    print(row)
    y = y + 1
```
