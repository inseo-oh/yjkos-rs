// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use std::{env, path::PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"));

    // These files affect the custom target's link step, but Cargo does not
    // otherwise know that the linker script is an input to the binary.
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("i586-yjkos.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("i586-yjkos.ld").display()
    );
}
