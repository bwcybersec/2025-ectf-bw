#!/bin/bash
. "$HOME/.cargo/env"

export CARGO_TARGET_DIR=target_docker

cargo build
# rust-objcopy -O binary --pad-to=0x10046000 target/thumbv7em-none-eabihf/debug/decoder /out/max78000.bin
rust-objcopy -O binary target_docker/thumbv7em-none-eabihf/debug/decoder /out/max78000.bin
cp target_docker/thumbv7em-none-eabihf/debug/decoder /out/max78000.elf