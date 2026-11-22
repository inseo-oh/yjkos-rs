# Repository Guidelines

## Project Structure

This repository is a Rust workspace for a freestanding i586 microkernel OS. `yjkernel/` contains the kernel and hardware abstraction layer; `yjkos/` contains the user program and syscall interface; `toolbox/` holds reusable utilities. Target specifications, linker scripts, and GRUB configuration live in `support/build/`. There are currently no dedicated `tests/` or asset directories; hardware-facing behavior is exercised by booting the image in QEMU.

## Build and Run

Install Rust nightly with the `rust-src` component, plus `build-essential`, `grub-pc-bin`, `xorriso`, and `qemu-system-x86`.

- `make` builds both targets and creates `yjkos_i586_debug.iso`.
- `make RELEASE=1` builds the release ISO.
- `make build-yjkos` or `make build-yjkernel` builds one target using its custom i586 target specification.
- `make run` boots the ISO in QEMU with serial output on the terminal.
- `make gdb` starts GDB attached to QEMU's default remote port (start QEMU with its GDB server enabled first).
- `make clean` removes Cargo outputs and generated ISOs.

Because the binaries are `no_std` and use custom targets, prefer the Makefile commands over host `cargo run` or `cargo test`.

## Coding Style

Follow standard Rust formatting: four-space indentation, `snake_case` for functions/modules, `UpperCamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Run `cargo +nightly fmt --all -- --check` before submitting. Keep hardware-specific code under `yjkernel/src/hal/i586/`; place shared kernel facilities under `yjkernel/src/klib/`. Preserve the existing copyright and SPDX headers in source files.

Keep Rust items private by default and expose them only as far as their callers require. Use `pub(super)` or `pub(in crate::...)` for module-scoped access, and `pub(crate)` for access across the crate. Use plain `pub` only for an intentional external API.

## Testing

There is no standalone test framework configured for the OS binaries. For changes affecting kernel or syscall behavior, build with `make`, boot with `make run`, and inspect kernel/user output for failures. Utility crates may gain Rust unit tests where they can run on the host, but target-specific paths should also be checked with the matching Makefile build.

## Commits and Pull Requests

Recent commits use short, imperative subjects, often naming the affected area (for example, `shell: fix ...` or `README: ...`). Keep commits focused and describe the behavior changed. Pull requests should explain the motivation, summarize implementation choices, list build or QEMU checks performed, and include relevant console output for boot/runtime changes. Link related issues when available.

## Configuration Notes

Custom target JSON and linker scripts determine the freestanding ABI and memory layout. Update them deliberately and validate with the corresponding Makefile target. Do not commit generated `target/` contents or ISO images.
