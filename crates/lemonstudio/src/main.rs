// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 thacer13
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Desktop executable. The application and platform shell live in `os_app`.

#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

fn main() {
    os_app::run();
}
