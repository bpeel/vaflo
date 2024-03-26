#!/bin/bash

set -eu

cd $(dirname "$0")

cargo run --quiet --bin=dump-dictionary data/dictionary.bin > before
for x in "$@"; do
    echo "$x"
done | cat before - | \
    cargo run --quiet --bin=make-dictionary data/dictionary.bin

cargo run --quiet --bin=dump-dictionary data/dictionary.bin > after

diff before after
