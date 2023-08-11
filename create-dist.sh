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

for x in "${files[@]}"; do
    dn="dist/$(dirname "$x")"
    bn="$(basename "$x")"
    mkdir -p "$dn"
    cp -v "$x" "$dn/$bn"
done
