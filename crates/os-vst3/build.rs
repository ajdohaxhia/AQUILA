// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

fn main() {
    println!("cargo:rerun-if-changed=vendor/ara/ARAInterface.h");
    bindgen::Builder::default()
        .header("vendor/ara/ARAInterface.h")
        .allowlist_type("ARA.*")
        .allowlist_var("kARA.*")
        .derive_default(true)
        .generate_comments(false)
        .layout_tests(true)
        .generate()
        .expect("ARA C bindings")
        .write_to_file(std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("ara.rs"))
        .unwrap();
}
