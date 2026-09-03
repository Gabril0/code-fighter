#!/usr/bin/env python3
"""Reference generator for Maximum Subarray Sum."""
import os
import random
import sys


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else int(os.environ.get("CASE_SEED", "0"))
    rng = random.Random(seed)

    count = rng.randint(1, 200)
    all_negative = rng.randint(1, 100) <= 15
    if all_negative:
        numbers = [rng.randint(-1000, -1) for _ in range(count)]
    else:
        numbers = [rng.randint(-1000, 1000) for _ in range(count)]

    print(count)
    print(" ".join(str(value) for value in numbers))


if __name__ == "__main__":
    main()
