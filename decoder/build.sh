#!/bin/bash
. "$HOME/.cargo/env"

export CARGO_TARGET_DIR=target_docker

cargo clean
cargo build --release
rust-objcopy -O binary target_docker/thumbv7em-none-eabihf/release/decoder /out/max78000.bin
cp target_docker/thumbv7em-none-eabihf/release/decoder /out/max78000.elf
# cargo build --profile=release-with-debug
# rust-objcopy -O binary target_docker/thumbv7em-none-eabihf/release/decoder /out/max78000.bin
# cp target_docker/thumbv7em-none-eabihf/release/decoder /out/max78000.elf
