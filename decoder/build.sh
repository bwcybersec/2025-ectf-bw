#!/bin/bash
. "$HOME/.cargo/env"
cargo build
# rust-objcopy -O binary --pad-to=0x10046000 target/thumbv7em-none-eabihf/debug/decoder /out/max78000.bin
rust-objcopy -O binary target/thumbv7em-none-eabihf/debug/decoder /out/max78000.bin
cp target/thumbv7em-none-eabihf/debug/decoder /out/max78000.elf