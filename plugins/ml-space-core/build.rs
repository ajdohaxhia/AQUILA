// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

fn main() {
    let dir = "vendor/FFTConvolver";
    let mut b = cc::Build::new();
    b.cpp(true).std("c++14").include(dir).warnings(false);
    for name in [
        "AudioFFT.cpp",
        "Utilities.cpp",
        "FFTConvolver.cpp",
        "TwoStageFFTConvolver.cpp",
    ] {
        b.file(format!("{dir}/{name}"));
    }
    b.file("src/convolution.cpp").compile("ml_convolution");
    println!("cargo:rerun-if-changed=vendor/FFTConvolver");
    println!("cargo:rerun-if-changed=src/convolution.cpp");
}
