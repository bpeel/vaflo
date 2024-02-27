#!/bin/bash

set -eu

cd $(dirname "$0")

../pucxobot/build/src/test-dictionary data/dictionary.bin > before
for x in "$@"; do
    echo "$x"
done | cat before - | \
    ../pucxobot/build/src/make-dictionary data/dictionary.bin

../pucxobot/build/src/test-dictionary data/dictionary.bin > after

diff before after
