#!/bin/bash

set -eu

files=(
    "empty-star.png"
    "filled-star.png"
    "index.html"
    "favicon.ico"
    "puzzles.txt"
    "donaci.svg"
    "drag.svg"
    "colors.svg"
    "cross.svg"
)

included_files=(
    "vaflo.js"
    "vaflo.css"
)

pkg_files=(
    "vaflo_bg.wasm"
    "vaflo.js"
)

for x in "${files[@]}" "${included_files[@]}"; do
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

for x in "${included_files[@]}"; do
    md5=$(md5sum "dist/$x" | sed 's/ .*//')
    new_name=$(echo "$x" | sed 's/\./'"-$md5"'./')
    mv "dist/$x" "dist/$new_name"
    re_filename=$(echo "$x" | sed 's/\./\\./g')
    sed -i s/\""$re_filename"\"/\""$new_name"\"/g dist/index.html
done
