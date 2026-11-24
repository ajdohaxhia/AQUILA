# Windows-hosted Linux build

This procedure describes the 2026103102 edition build. Current AQUILA Linux releases are built natively on Ubuntu 24.04 and require glibc 2.39 or newer.

Build target: x86_64-unknown-linux-gnu.2.36, Rust 1.99.1, cargo-zigbuild 0.24.0, Zig 0.13.0. Install these from the official Rust and PyPI distributions. Add the x86_64-unknown-linux-gnu Rust target. Obtain libasound2-dev and libasound2 from Debian bookworm, verify package SHA-256 against its Packages index, and extract into an isolated sysroot. Preserve libasound.so as a linker name for libasound.so.2.0.0.

Use ASCII, space-free paths for Zig and generated wrappers (a temporary subst drive on Windows works). Set CARGO_ZIGBUILD_ZIG_PATH to zig.exe, CARGO_ZIGBUILD_CACHE_DIR to an isolated cache, PKG_CONFIG to a pkgconf wrapper that adds --dont-define-prefix, PKG_CONFIG_ALLOW_CROSS=1, and PKG_CONFIG_LIBDIR to the sysroot's ALSA pkgconfig directory. Set alsa.pc prefix/libdir/includedir to the actual sysroot paths. Do not let Windows pkgconf relocate the multiarch prefix automatically. Set OPENSTUDIO_NO_PLUGIN_BUILD=1, then `cargo-zigbuild zigbuild --locked --release -p os-app --target x86_64-unknown-linux-gnu.2.36`.

The 2026103102 edition used a patch in `vendor/mp3lame-sys` for host-dependent LAME headers. Current AQUILA builds use `mp3lame-sys` 0.2 with LAME 4.0. The build script compiles with `cc` for Cargo's target, and the C header uses the target compiler's platform macros and ABI sizes. The Windows-to-Linux procedure was verified only for the earlier edition.

Package the ELF executable in tar.gz with mode 0755, plus README, LICENSE and THIRD-PARTY-NOTICES. Keep matching source available at the same download page. Do not call this Linux-tested: release 2026103102 was not run on Linux.
