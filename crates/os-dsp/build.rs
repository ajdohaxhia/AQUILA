// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

fn main() {
 let dir="vendor/snaptune";
 println!("cargo:rerun-if-changed={dir}");
 cc::Build::new().cpp(true).std("c++17").include(dir)
 .file(format!("{dir}/AutoTuneEngine.cpp"))
 .file(format!("{dir}/PitchDetector.cpp"))
 .file(format!("{dir}/bridge.cpp"))
 .flag_if_supported("/EHsc").compile("lemon_snaptune");
}
