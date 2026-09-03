#!/usr/bin/env python3
"""Reference solver for Word Frequency Count. JSON output."""
import json
import re
import sys


def main():
    text = sys.stdin.read().lower()
    words = re.findall(r"[a-z0-9]+", text)

    counts = {}
    for word in words:
        counts[word] = counts.get(word, 0) + 1

    print(json.dumps(counts, indent=2, ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
