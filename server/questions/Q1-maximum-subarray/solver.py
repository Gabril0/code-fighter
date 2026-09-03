#!/usr/bin/env python3
"""Reference solver for Maximum Subarray Sum (Kadane's algorithm)."""
import sys


def main():
    tokens = sys.stdin.read().split()
    count = int(tokens[0])
    numbers = [int(value) for value in tokens[1:1 + count]]

    best = current = numbers[0]
    for value in numbers[1:]:
        current = max(value, current + value)
        best = max(best, current)

    print(best)


if __name__ == "__main__":
    main()
