// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

fn main() {
    println!("cargo:rerun-if-changed=vendor/velociloops");
    println!("cargo:rerun-if-changed=src/rex_bridge.cpp");
    cc::Build::new().cpp(true).std("c++17").warnings(false)
        .include("vendor/velociloops").file("src/rex_bridge.cpp")
        .flag_if_supported("/EHsc").compile("ml_rex");
}
