#!/usr/bin/env python3
"""Reference solver for FizzBuzz."""
import sys


def main():
    count = int(sys.stdin.read().split()[0])

    lines = []
    for value in range(1, count + 1):
        if value % 15 == 0:
            lines.append("FizzBuzz")
        elif value % 3 == 0:
            lines.append("Fizz")
        elif value % 5 == 0:
            lines.append("Buzz")
        else:
            lines.append(str(value))

    sys.stdout.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
