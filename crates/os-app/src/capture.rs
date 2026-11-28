// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! GUI access to the session's shared audio capture.
pub use os_session::capture::*;

pub fn last_input() -> Option<Input> {
    crate::settings_file::get("rec_input").as_deref().and_then(Input::parse)
}

pub fn set_last_input(input: &Input) {
    crate::settings_file::write("rec_input", &input.key());
}

/// Uses the desktop's active ASIO driver, whose device state belongs to the desktop transport.
pub fn devices() -> Vec<(String, bool, u16)> {
    let cfg = crate::audio_cfg::AudioCfg::current();
    if let Some(name) = cfg.asio_driver() {
        let opened = crate::device::last()
            .filter(|device| device.host == crate::device::ASIO && device.name == name)
            .map(|device| device.inputs);
        return vec![(name.to_string(), true, opened.unwrap_or(2))];
    }
    os_session::capture::devices()
}
