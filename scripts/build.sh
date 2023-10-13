#!/bin/bash

set -e
RUSTFLAGS="-C target-feature=+crt-static -C relocation-model=static" \
cargo lambda build \
-j 2 \
--target x86_64-unknown-linux-musl  \
--release \
--output-format Zip
