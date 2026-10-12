#!/usr/bin/env sh

cross build --target x86_64-unknown-linux-gnu --release
cross build --target x86_64-pc-windows-gnu --release

sha256sum target/x86_64-unknown-linux-gnu/release/cls
sha256sum target/x86_64-pc-windows-gnu/release/cls.exe