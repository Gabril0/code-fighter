#!/usr/bin/env python3
"""Reference generator for FizzBuzz."""
import os
import random
import sys


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else int(os.environ.get("CASE_SEED", "0"))
    rng = random.Random(seed)

    # Bias toward small N most of the time, occasionally a larger one.
    if rng.randint(1, 100) <= 20:
        count = rng.randint(1, 100000)
    else:
        count = rng.randint(1, 100)

    print(count)


if __name__ == "__main__":
    main()
