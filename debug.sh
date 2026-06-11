#!/bin/bash

# qemu-test.sh - Automates testing tinyinit in QEMU
# Usage: ./qemu-test.sh [--no-clean] [--kernel KERNEL_PATH]

set -e

# config
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
INITRD_DIR="/tmp/tinyinit-initrd"
KERNEL="${KERNEL:-/boot/vmlinuz-linux}"
BUSYBOX_URL="https://busybox.net/downloads/binaries/1.35.0-x86_64-linux-musl/busybox"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# parse args
NO_CLEAN=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-clean) NO_CLEAN=true; shift ;;
        --kernel) KERNEL="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo -e "${GREEN}=== tinyinit QEMU Test Script ===${NC}"

# 1. build init 
echo -e "\n${YELLOW}[1/6] Building tinyinit (static musl)...${NC}"
cd "$PROJECT_DIR"
rustup target add x86_64-unknown-linux-musl >/dev/null 2>&1 || true
cargo build --release --target=x86_64-unknown-linux-musl 2>&1 | tail -5

if [ ! -f "target/x86_64-unknown-linux-musl/release/tinyinit" ]; then
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi
echo -e "${GREEN}Build successful${NC}"

# 2. download busybox
echo -e "\n${YELLOW}[2/6] Setting up busybox...${NC}"
if [ ! -f "$PROJECT_DIR/busybox" ]; then
    wget -q --show-progress "$BUSYBOX_URL" -O "$PROJECT_DIR/busybox"
    chmod +x "$PROJECT_DIR/busybox"
fi
echo -e "${GREEN}Busybox ready${NC}"

# 3. initramfs dir
echo -e "\n${YELLOW}[3/6] Creating initramfs...${NC}"
if [ "$NO_CLEAN" = false ] && [ -d "$INITRD_DIR" ]; then
    rm -rf "$INITRD_DIR"
fi
mkdir -p "$INITRD_DIR"

# 4. copy bins
cp "$PROJECT_DIR/target/x86_64-unknown-linux-musl/release/tinyinit" "$INITRD_DIR/init"
cp "$PROJECT_DIR/busybox" "$INITRD_DIR/busybox"
chmod +x "$INITRD_DIR/init" "$INITRD_DIR/busybox"

echo -e "${GREEN}Initramfs prepared at $INITRD_DIR${NC}"

# 5. create initramfs archive
echo -e "\n${YELLOW}[4/6] Packing initramfs...${NC}"
cd "$INITRD_DIR"
find . -print0 | cpio -o -0 -H newc --quiet | gzip > /tmp/tinyinit-initramfs.cpio.gz
cd - > /dev/null
echo -e "${GREEN}Initramfs created: /tmp/tinyinit-initramfs.cpio.gz${NC}"

# 6. verify kernel (i never needed to run this on any other kernel besides the default arch kernel, but do modify this as needed)
echo -e "\n${YELLOW}[5/6] Verifying kernel...${NC}"
if [ ! -f "$KERNEL" ]; then
    echo -e "${RED}Kernel not found at $KERNEL${NC}"
    echo -e "Specify with: --kernel /path/to/vmlinuz"
    exit 1
fi
echo -e "${GREEN}Using kernel: $KERNEL${NC}"

# 7. launches QEMU
echo -e "\n${YELLOW}[6/6] Launching QEMU...${NC}"
echo -e "${GREEN}Press Ctrl+A then X to exit QEMU${NC}"
echo -e "${GREEN}Inside shell, type commands (use /busybox ps, /busybox top)${NC}"
echo -e "${GREEN}Type 'exit' to shut down${NC}\n"

qemu-system-x86_64 \
    -kernel "$KERNEL" \
    -initrd /tmp/tinyinit-initramfs.cpio.gz \
    -append "init=/init console=ttyS0" \
    -nographic \
    -no-reboot \
    -m 256M \
    -cpu max

# THIS IS FOR OPENING A NEW WINDOW
# qemu-system-x86_64 \
#     -kernel /boot/vmlinuz-linux \
#     -initrd /tmp/tinyinit-initramfs.cpio.gz \
#     -append "init=/init console=tty0" \
#     -m 256M \
#     -cpu max \
#     -vga std \
#     -display gtk


# cleanup
echo -e "\n${YELLOW}QEMU exited. Cleaning up...${NC}"
if [ "$NO_CLEAN" = false ]; then
    rm -rf "$INITRD_DIR"
    rm -f /tmp/tinyinit-initramfs.cpio.gz
    echo -e "${GREEN}Cleanup complete${NC}"
fi