#!/usr/bin/env sh

cargo build --target x86_64-unknown-linux-gnu --release
cargo build --target x86_64-pc-windows-gnu --release