# YJK/OS

Microkernel OS project that started as Rust port of [YJK/OS](https://github.com/inseo-oh/yjkos).
Currently it's in very early WIP stage.

## AI disclaimer

Majority of code is my own code(except test code), but I do get some help from AI chatbots, and they also write most of test code.

## Build

Install the system dependencies:

```sh
sudo apt install build-essential grub-pc-bin xorriso qemu-system-x86
```

Building also requires Rust nightly with the `rust-src` component.

```sh
make RELEASE=1 # Omit RELEASE=1 if you want debug build
```

The bootable ISO is written to `yjkos_i586_release.iso` (or `yjkos_i586_debug.iso` for debug build).

## Run

With QEMU installed:

```sh
make run
```

## Recommended settings for VSCode (JSON)

```json
{
    "rust-analyzer.cargo.target": "support/build/i586-yjkos.json",
    "rust-analyzer.cargo.extraArgs": [
        "-Zjson-target-spec",
        "-Zbuild-std=core,alloc"
    ],
    "rust-analyzer.cargo.extraEnv": {
        "RUSTUP_TOOLCHAIN": "nightly"
    },
    "rust-analyzer.cargo.allTargets": false
}
```
