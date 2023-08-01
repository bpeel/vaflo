#!/usr/bin/python3

import sys
import json
import base64
import brotli
import subprocess

puzzles = json.load(sys.stdin)

abnormal_results = []

for (day, puzzle) in puzzles.items():
    puzzle = json.loads(brotli.decompress(base64.b64decode(puzzle)))
    print(f"Day {day}")
    output = subprocess.check_output(["target/release/solve-swap",
                                      puzzle["puzzle"],
                                      puzzle["solution"]],
                                     encoding="utf-8")
    print(output, end='')

    if not output.startswith("10 swaps"):
        abnormal_results.append((day, output))

if len(abnormal_results) > 0:
    print("\nAbnormal results:\n")

    for (day, output) in abnormal_results:
        print(f"Day {day}\n{output}", end='')
