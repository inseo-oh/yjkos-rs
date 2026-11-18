#!/bin/sh
set -ue

TOOLCHAIN_BINDIR=../yjkos/toolchain/bin
BOOTROOT=$(mktemp -d --suffix=-yjkos-bootroot)
rm -f support
ln -s ../yjkos/support support

cargo +nightly build -Z build-std=core,alloc -Zjson-target-spec --target i586-yjkos.json --release

mkdir -p $BOOTROOT $BOOTROOT/boot $BOOTROOT/boot/grub
cp support/res/grub.cfg $BOOTROOT/boot/grub/

cp target/i586-yjkos/release/yjkos-rs $BOOTROOT/boot/yjkernel
mkdir -p out/i586/
$TOOLCHAIN_BINDIR/grub-mkrescue -o out/i586/YJKOS_i586.iso $BOOTROOT


