#!/bin/bash

# Run BattleRoyaleOS in QEMU

set -e

# Change to project root
cd "$(dirname "$0")/.."

# Build first
echo "Building..."
cargo build --release

# Check if ISO needs to be built
if [ ! -f "image.iso" ] || [ "target/x86_64-unknown-none/release/kernel" -nt "image.iso" ]; then
    echo "Building ISO..."
    make iso
fi

# Run QEMU
echo "Starting QEMU..."
qemu-system-x86_64 \
    -M q35 \
    -m 512M \
    -smp 5 \
    -cdrom image.iso \
    -serial stdio \
    -device e1000,netdev=net0 \
    -netdev user,id=net0,hostfwd=udp::5000-:5000 \
    -no-reboot \
    "$@"
