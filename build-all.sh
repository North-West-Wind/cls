#!/usr/bin/env sh

cargo build --target x86_64-unknown-linux-gnu --release
cargo build --target x86_64-pc-windows-gnu --release

sha256sum target/x86_64-unknown-linux-gnu/release/cls
sha256sum target/x86_64-pc-windows-gnu/release/cls.exe