#!/bin/bash

set -eu

host=$(sed -nr '/^\s*pub fn share_text\b/,/^    }/ s|\s*https://(.*)".*|\1|p' \
       src/save_state.rs)

ncftpput bdn "/$host" puzzles.txt
