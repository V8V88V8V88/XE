#!/usr/bin/env python3

import argparse
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def fib(n: int) -> int:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)


def run_python_workload() -> str:
    values = [fib(35), fib(38)]
    return "\n".join(str(value) for value in values) + "\n"


def time_command(command: list[str], runs: int) -> tuple[float, str]:
    samples = []
    stdout = None

    for _ in range(runs):
        started = time.perf_counter()
        completed = subprocess.run(
            command,
            check=True,
            capture_output=True,
            text=True,
        )
        samples.append(time.perf_counter() - started)
        if stdout is None:
            stdout = completed.stdout
        elif completed.stdout != stdout:
            raise RuntimeError("benchmark command produced inconsistent output across runs")

    return statistics.mean(samples), stdout or ""


def ensure_xe_compiler(repo_root: Path) -> Path:
    compiler = repo_root / "target" / "release" / ("xe.exe" if sys.platform == "win32" else "xe")
    if compiler.exists():
        return compiler

    subprocess.run(
        ["cargo", "build", "--release", "--locked"],
        cwd=repo_root,
        check=True,
    )
    return compiler


def benchmark_xe(repo_root: Path, runs: int) -> tuple[float, str]:
    compiler = ensure_xe_compiler(repo_root)
    source = repo_root / "examples" / "benchmark.xe"

    with tempfile.TemporaryDirectory(prefix="xe-benchmark-") as temp_dir:
        temp_path = Path(temp_dir)
        binary = temp_path / ("benchmark.exe" if sys.platform == "win32" else "benchmark")

        subprocess.run(
            [str(compiler), "compile", str(source), "-o", str(binary)],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        )

        return time_command([str(binary)], runs)


def benchmark_python(runs: int) -> tuple[float, str]:
    return time_command([sys.executable, __file__, "--python-workload"], runs)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Benchmark XE against CPython using the same Fibonacci workload."
    )
    parser.add_argument(
        "--runs",
        type=int,
        default=3,
        help="number of timed runs per implementation (default: 3)",
    )
    parser.add_argument(
        "--python-workload",
        action="store_true",
        help=argparse.SUPPRESS,
    )
    args = parser.parse_args()

    if args.python_workload:
        sys.stdout.write(run_python_workload())
        return 0

    repo_root = Path(__file__).resolve().parents[1]
    xe_time, xe_output = benchmark_xe(repo_root, args.runs)
    python_time, python_output = benchmark_python(args.runs)

    if xe_output != python_output:
        print("Benchmark outputs do not match.", file=sys.stderr)
        return 1

    ratio = python_time / xe_time if xe_time > 0 else float("inf")

    print("XE vs Python benchmark")
    print("Workload: recursive Fibonacci (fib(35), fib(38))")
    print(f"Runs per implementation: {args.runs}")
    print()
    print(f"{'Implementation':<16}{'Mean time (s)':>16}")
    print(f"{'-' * 16}{'-' * 16:>16}")
    print(f"{'XE':<16}{xe_time:>16.4f}")
    print(f"{'Python':<16}{python_time:>16.4f}")
    print()
    print(f"Speedup (Python / XE): {ratio:.2f}x")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
