def run_bench():
    total = 0.0
    for _ in range(100000000):
        total += 1.0
    return total

print(run_bench())
