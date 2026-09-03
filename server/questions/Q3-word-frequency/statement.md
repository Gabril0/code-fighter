# Word Frequency Count

Given a block of text, count how many times each **word** appears and
return the result as a JSON object.

A **word** is a maximal run of the characters `a`–`z` and `0`–`9`, matched
**case-insensitively**. Before counting, convert every letter to
lowercase. Any other character (spaces, punctuation, newlines, etc.) is a
separator and is never part of a word.

## Input

One or more lines of arbitrary text, read until end of input.

## Output

A JSON object whose keys are the distinct lowercased words and whose
values are their integer counts. Key order does not matter and formatting
(whitespace/indentation) is ignored — only the keys and their counts are
compared.

## Example

Input:
```
The quick brown fox, the QUICK dog!
```

Output:
```json
{
  "brown": 1,
  "dog": 1,
  "fox": 1,
  "quick": 2,
  "the": 2
}
```
