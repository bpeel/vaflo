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
)

pkg_files=(
    "vaflo_bg.wasm"
    "vaflo.js"
)

for x in "${files[@]}"; do
    dn="dist/$(dirname "$x")"
    bn="$(basename "$x")"
    mkdir -p "$dn"
    cp -v "$x" "$dn/$bn"
done

pkg_md5=$(cat "${pkg_files[@]/#/pkg\//}" | md5sum - | sed 's/ .*//')
pkg_dir="dist/pkg-$pkg_md5"

mkdir -p "$pkg_dir"

for x in "${pkg_files[@]}"; do
    cp -v "pkg/$x" "$pkg_dir/$x"
done

sed -i 's|\./pkg/vaflo\.js|./pkg-'"$pkg_md5"'/vaflo.js|' dist/vaflo.js
