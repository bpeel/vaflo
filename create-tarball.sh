#!/bin/bash

set -eu

files=(
    "empty-star.png"
    "filled-star.png"
    "index.html"
    "favicon.ico"
    "puzzles.txt"
    "vaflo.js"
    "vaflo.css"
    "pkg/vaflo_bg.wasm"
    "pkg/vaflo.js"
)

tar -jvcf vaflo.tar.bz2 "${files[@]}"
