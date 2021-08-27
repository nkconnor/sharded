#!/bin/sh
RUSTFLAGS="-C target-cpu=native"
cargo run --release --bin main > latest.txt
