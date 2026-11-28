// Copyright (C) 2026 ItsAlphaNeon
// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Shared session plugin host and desktop editor integration.

pub use os_session::plugins::*;
pub use crate::editor_host::{ChannelInfoT, Request};
pub(crate) use crate::editor_host::{editor_action, wave_zoom};
#[cfg(target_os = "macos")]
pub mod window_check;

/// Resolves a sample with the desktop application's active settings.
pub fn resolve_sample(path: &str) -> String {
    os_session::plugins::resolve_sample(&crate::settings_file::store(), path)
}

/// Builds the sampler state using the desktop application's active settings.
pub fn sampler_state(channel: &os_model::Channel) -> Vec<u8> {
    os_session::plugins::sampler_state(&crate::settings_file::store(), channel)
}

/// Saves a sampler state using the desktop application's active settings.
pub fn store_sampler(channel: &mut os_model::Channel, state: &[u8]) {
    os_session::plugins::store_sampler(&crate::settings_file::store(), channel, state);
}

pub use os_session::presets::WindowEdit;
