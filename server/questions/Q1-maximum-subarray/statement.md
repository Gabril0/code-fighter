# Maximum Subarray Sum

Given an array of integers, find the contiguous subarray (containing at
least one element) that has the largest sum, and return that sum.

This is the classic problem solved by **Kadane's algorithm** in O(n).

## Input

- The first line contains an integer `N` — the number of elements.
- The second line contains `N` space-separated integers (each may be
  negative, zero, or positive).

## Output

A single line with one integer: the maximum sum of any contiguous
subarray.

## Example

Input:
```
9
-2 1 -3 4 -1 2 1 -5 4
```

Output:
```
6
```

The subarray `[4, -1, 2, 1]` has the largest sum, `6`.

Note: when every element is negative, the answer is the largest single
element (a subarray must contain at least one element).
