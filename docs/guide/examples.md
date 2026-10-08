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

## Language Features in Action

These examples show the features added in XE 0.2.0.

### Word Counter

Source: `examples/word_count.xe`

Counts words in a text: text functions (`lower`, `replace`, `split`), a map that keeps words in the order they first appeared, `in`, `+=`, slicing, and `sort`.

```xe
# Word counter: text functions, maps that keep insertion order, and sorting.

text = "The quick brown fox jumps over the lazy dog. The dog sleeps, the fox runs!"

# Normalize: lowercase and strip punctuation
clean = lower(text)
for mark in [".", ",", "!", "?"]:
    clean = replace(clean, mark, "")

words = clean.split()
counts = {}
for word in words:
    if word in counts:
        counts[word] += 1
    else:
        counts[word] = 1

print("Words:", length(words), "- unique:", length(counts))

# Words in the order they first appeared
print("First seen:", keys(counts)[:5])

# The three most common words
remaining = copy(counts)
repeat 3 times:
    best = none
    for word in remaining:
        if best == none or remaining[word] > remaining[best]:
            best = word
    print(best + ":", remaining[best])
    remove(remaining, best)

# Alphabetical list of words used once
once = []
for word in counts:
    if counts[word] == 1:
        once.append(word)
print("Used once:", join(sort(once), ", "))
```

Output:

```text
Words: 15 - unique: 10
First seen: ["the", "quick", "brown", "fox", "jumps"]
the: 4
fox: 2
dog: 2
Used once: brown, jumps, lazy, over, quick, runs, sleeps
```

### Bank Account

Source: `examples/bank_account.xe`

Structs with a list field, shared values (functions change the account passed to them), and `try` / `catch` with `error(...)` so one failed request does not stop the program.

```xe
# Bank account: structs, shared values, and error handling with try / catch.

struct Account:
    owner
    balance
    history

fun open_account(owner, amount):
    return Account(owner, amount, ["opened with " + convert(amount, "text")])

fun deposit(account, amount):
    if amount <= 0:
        error("deposit must be positive")
    account.balance += amount
    account.history.append("deposit " + convert(amount, "text"))

fun withdraw(account, amount):
    if amount > account.balance:
        error("insufficient funds: balance is " + convert(account.balance, "text"))
    account.balance -= amount
    account.history.append("withdraw " + convert(amount, "text"))

fun transfer(source, target, amount):
    withdraw(source, amount)
    deposit(target, amount)

alice = open_account("Alice", 100)
bob = open_account("Bob", 20)

transfer(alice, bob, 30)
print(alice.owner, alice.balance, "|", bob.owner, bob.balance)

# Each request either succeeds or is reported, and the program keeps going
requests = [
    {"account": bob, "action": "withdraw", "amount": 500},
    {"account": alice, "action": "deposit", "amount": -5},
    {"account": alice, "action": "withdraw", "amount": 50}
]
for request in requests:
    account = request.account
    try:
        if request.action == "deposit":
            deposit(account, request.amount)
        else:
            withdraw(account, request.amount)
        print("ok:", account.owner, "now has", account.balance)
    catch err:
        print("failed for", account.owner + ":", err)

print("Alice history:", alice.history)
```

Output:

```text
Alice 70 | Bob 50
failed for Bob: insufficient funds: balance is 50
failed for Alice: deposit must be positive
ok: Alice now has 20
Alice history: ["opened with 100", "withdraw 30", "withdraw 50"]
```

### Functional Toolkit

Source: `examples/functional.xe`

Functions as values: `map`, `filter` and `reduce` written in XE, functions that return functions (`make_multiplier`, `compose`), built-ins passed as values, and a map of lambdas.

```xe
# Functions are values: pass them around, return them, and write them inline with lambda.

fun map_list(f, items):
    out = []
    for item in items:
        out.append(f(item))
    return out

fun filter_list(keep, items):
    out = []
    for item in items:
        if keep(item):
            out.append(item)
    return out

fun reduce(f, items, start):
    total = start
    for item in items:
        total = f(total, item)
    return total

fun compose(f, g):
    return lambda x: f(g(x))

fun make_multiplier(factor):
    return lambda x: x * factor

numbers = range(1, 11)

squares = map_list(lambda n: n ** 2, numbers)
evens = filter_list(lambda n: n % 2 == 0, numbers)
total = reduce(lambda a, b: a + b, numbers, 0)
print("squares:", squares)
print("evens:", evens)
print("sum:", total, "- same as sum():", sum(numbers))

triple = make_multiplier(3)
triple_then_square = compose(lambda x: x * x, triple)
print("triple(4) =", triple(4), "- square(triple(4)) =", triple_then_square(4))

# Built-in functions are values too
words = ["xe", "lambda", "map"]
print(map_list(upper, words), map_list(length, words))

# A table of operations
operations = {
    "add": lambda a, b: a + b,
    "sub": lambda a, b: a - b,
    "pow": lambda a, b: a ** b
}
for name in operations:
    print(name, operations[name](2, 5))
```

Output:

```text
squares: [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]
evens: [2, 4, 6, 8, 10]
sum: 55 - same as sum(): 55
triple(4) = 12 - square(triple(4)) = 144
["XE", "LAMBDA", "MAP"] [2, 6, 3]
add 7
sub -3
pow 32
```

### Journal

Source: `examples/journal.xe`

Reads command-line arguments with `args()` and keeps a journal in a text file with `file_exists`, `write_file`, `append_file` and `read_file`.

```xe
# Journal: files and command-line arguments.
# Usage: xe run examples/journal.xe <file> [entry...]

fun count_of(n, singular, plural):
    if n == 1:
        return "1 " + singular
    return convert(n, "text") + " " + plural

if length(args()) == 0:
    print("usage: xe run examples/journal.xe <file> [entry...]")
    exit(1)

path = args()[0]
entries = args()[1:]

if not file_exists(path):
    write_file(path, "# Journal\n")
    print("Created", path)

for entry in entries:
    append_file(path, "- " + entry + "\n")
print("Added", count_of(length(entries), "entry", "entries"))

lines = split(trim(read_file(path)), "\n")
items = lines[1:]
print("The journal has", count_of(length(items), "entry", "entries") + ":")
for i in range(length(items)):
    print(convert(i + 1, "text") + ".", items[i][2:])
```

Run it with a file name and some entries:

```bash
xe run examples/journal.xe journal.txt "learned XE" "wrote a parser"
xe run examples/journal.xe journal.txt "fixed a bug"
```

Output of the second run:

```text
Added 1 entry
The journal has 3 entries:
1. learned XE
2. wrote a parser
3. fixed a bug
```

### Game of Life

Source: `examples/game_of_life.xe`

Conway's Game of Life on a small board: nested lists, `range`, `continue`, and `%` that wraps negative numbers around (`(0 - 1) % 6` is `5`).

```xe
# Conway's Game of Life: nested lists, range, and a board that wraps around.

fun make_board(rows, cols, cells):
    board = []
    for r in range(rows):
        row = []
        for c in range(cols):
            row.append([r, c] in cells)
        board.append(row)
    return board

fun live_neighbours(board, r, c):
    rows = length(board)
    cols = length(board[0])
    count = 0
    for dr in [-1, 0, 1]:
        for dc in [-1, 0, 1]:
            if dr == 0 and dc == 0:
                continue
            # % wraps around: (0 - 1) % rows is the last row
            if board[(r + dr) % rows][(c + dc) % cols]:
                count += 1
    return count

fun step(board):
    next_board = []
    for r in range(length(board)):
        row = []
        for c in range(length(board[r])):
            n = live_neighbours(board, r, c)
            alive = board[r][c]
            row.append(n == 3 or (alive and n == 2))
        next_board.append(row)
    return next_board

fun show(board, generation):
    print("Generation", generation)
    for row in board:
        line = ""
        for cell in row:
            if cell:
                line += "#"
            else:
                line += "."
        print(line)

glider = [[0, 1], [1, 2], [2, 0], [2, 1], [2, 2]]
board = make_board(6, 6, glider)
for generation in range(4):
    show(board, generation)
    board = step(board)
```

The first two generations of the output:

```text
Generation 0
.#....
..#...
###...
......
......
......
Generation 1
......
#.#...
.##...
.#....
......
......
```
