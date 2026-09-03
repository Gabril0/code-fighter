#!/usr/bin/env python3
"""Reference generator for Word Frequency Count."""
import os
import random
import sys

VOCAB = [
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
    "code", "fighter", "ring", "punch", "data", "json", "test", "array",
    "graph", "queue", "stack", "hash", "tree", "node", "edge", "alpha",
    "beta", "gamma", "delta", "42", "loop", "byte",
]
SEPARATORS = [" ", "  ", ", ", ". ", "! ", "\n", "; ", " - "]


def main():
    seed = int(sys.argv[1]) if len(sys.argv) > 1 else int(os.environ.get("CASE_SEED", "0"))
    rng = random.Random(seed)

    count = rng.randint(3, 120)
    pieces = []
    for index in range(count):
        word = rng.choice(VOCAB)
        # Randomly upper-case some letters to exercise case folding.
        if rng.randint(1, 100) <= 30:
            word = word.upper()
        pieces.append(word)
        if index < count - 1:
            pieces.append(rng.choice(SEPARATORS))

    sys.stdout.write("".join(pieces) + "\n")


if __name__ == "__main__":
    main()
