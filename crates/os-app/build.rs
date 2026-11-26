// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Embeds `assets/lemonstudio.ico` (a nightly's: `lemonstudio-nightly.ico`) as icon resource 1 and the version
//! (`LEMON_VERSION`, else the workspace's) as the version resource (Windows). Uses the Windows SDK's `rc.exe` (or `$RC`);
//! without it the build goes on and the exe just has no icon. Also sets `cfg(plugin_windows)` where plugin windows exist,
//! `cfg(lemon_updater)` when `LEMON_UPDATER` names the updater to carry (release builds, `crate::update`), and
//! `cfg(lemon_nightly)` when `LEMON_CHANNEL` is `nightly` (`crate::version`).

use std::path::PathBuf;
/// The factory presets in `factory-presets/<plugin>/[<folder>/]<name>.ospreset`, as a Rust slice expression of (path
/// under that folder, `/`-separated; bytes) for `presets.rs` to include (spec: plugins/presets.md "Factory presets").
fn factory_presets() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let roots = [manifest.join("factory-presets"), manifest.join("../os-session/factory-presets")];
    for root in &roots {
        println!("cargo:rerun-if-changed={}", root.display());
    }
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    let files = |dir: &std::path::Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        v.sort();
        v
    };
    let name = |p: &std::path::Path| p.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let preset = |p: &std::path::Path| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("ospreset"));
    for plugin in roots.iter().flat_map(|root| files(root)).filter(|p| p.is_dir()) {
        for p in files(&plugin) {
            if preset(&p) {
                found.push((format!("{}/{}", name(&plugin), name(&p)), p));
            } else if p.is_dir() {
                for q in files(&p).into_iter().filter(|q| preset(q)) {
                    found.push((format!("{}/{}/{}", name(&plugin), name(&p), name(&q)), q));
                }
            }
        }
    }
    let mut out = String::from("&[\n");
    for (rel, path) in found {
        out.push_str(&format!("    ({rel:?}, include_bytes!({:?}) as &[u8]),\n", path.display().to_string()));
    }
    out.push(']');
    std::fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("factory_presets.rs"), out).unwrap();
}

/// Release builds: `LEMON_UPDATER` names the `lemonstudio-updater` built just before, which the program carries
/// (`update::UPDATER`, through `LEMON_UPDATER_FILE`). A build from source has none.
fn updater() {
    println!("cargo::rustc-check-cfg=cfg(lemon_updater)");
    println!("cargo:rerun-if-env-changed=LEMON_UPDATER");
    let Some(path) = std::env::var_os("LEMON_UPDATER").filter(|p| !p.is_empty()) else { return };
    let path = std::path::absolute(PathBuf::from(path)).expect("LEMON_UPDATER");
    assert!(path.is_file(), "LEMON_UPDATER names no file: {}", path.display());
    println!("cargo:rerun-if-changed={}", path.display());
    println!("cargo::rustc-env=LEMON_UPDATER_FILE={}", path.display());
    println!("cargo::rustc-cfg=lemon_updater");
}

fn main() {
    println!("cargo:rerun-if-changed=vendor/minibpm");
    cc::Build::new().cpp(true).std("c++11")
        .file("vendor/minibpm/MiniBpm.cpp")
        .file("vendor/minibpm/lemon_bridge.cpp")
        .warnings(false).compile("lemon_minibpm");

    println!("cargo:rerun-if-env-changed=LEMON_MASTER_BUNDLE");
    println!("cargo:rerun-if-env-changed=LEMON_RELEASE");
    let bundle = std::env::var_os("LEMON_MASTER_BUNDLE").map(PathBuf::from);
    let bytes = bundle.as_ref().map(|p| {
        println!("cargo:rerun-if-changed={}", p.display());
        std::fs::read(p).expect("Cannot read Master Assistant bundle")
    }).unwrap_or_default();
    if std::env::var_os("LEMON_RELEASE").is_some() {
        assert!(std::env::var_os("CARGO_FEATURE_MASTER_ASSISTANT").is_some() && !bytes.is_empty(),
            "Release requires Master Assistant and LEMON_MASTER_BUNDLE");
    }
    std::fs::write(PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("master-engine.zip"), bytes).unwrap();
    let score_out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("score-engine.zip");
    println!("cargo:rerun-if-env-changed=LEMON_SCORE_BUNDLE");
    if let Some(path) = std::env::var_os("LEMON_SCORE_BUNDLE") {
        println!("cargo:rerun-if-changed={}", PathBuf::from(&path).display());
        std::fs::copy(path, score_out).expect("Could not embed Verovio score bundle");
    } else {
        std::fs::write(score_out, []).unwrap();
    }
    println!("cargo:rerun-if-env-changed=LEMON_VISUALIZER_WORKER");
    let worker_out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("visualizer-worker.exe");
    let windows_x64 = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86_64");
    if let Some(path) = std::env::var_os("LEMON_VISUALIZER_WORKER").filter(|_| windows_x64) {
        let path = PathBuf::from(path);
        println!("cargo:rerun-if-changed={}", path.display());
        std::fs::copy(path, worker_out).expect("Could not embed the Visualizer renderer");
    } else {
        std::fs::write(worker_out, []).unwrap();
    }
    match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => {
            println!("cargo:rerun-if-changed=src/sample_drag_windows.cpp");
            cc::Build::new().cpp(true).file("src/sample_drag_windows.cpp").compile("sample_drag");
            for lib in ["shell32", "ole32", "uuid"] { println!("cargo:rustc-link-lib={lib}"); }
        }
        Ok("macos") => {
            println!("cargo:rerun-if-changed=src/sample_drag_macos.m");
            cc::Build::new().flag("-fobjc-arc").file("src/sample_drag_macos.m").compile("sample_drag");
        }
        _ => {}
    }
    factory_presets();
    updater();
    println!("cargo:rerun-if-env-changed=LEMON_VERSION");
    println!("cargo:rerun-if-env-changed=LEMON_CHANNEL");
    println!("cargo::rustc-check-cfg=cfg(lemon_nightly)");
    if std::env::var("LEMON_CHANNEL").is_ok_and(|value| value == "nightly") {
        println!("cargo::rustc-cfg=lemon_nightly");
    }

    // `plugin_windows`: plugin editors open in AQUILA's frame windows (Win32 on Windows, X11 on Linux, AppKit on
    // macOS).
    println!("cargo::rustc-check-cfg=cfg(plugin_windows)");
    if matches!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("windows" | "linux" | "macos")) {
        println!("cargo::rustc-cfg=plugin_windows");
    }
}
