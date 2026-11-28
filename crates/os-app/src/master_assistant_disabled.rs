// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Public builds deliberately exclude the private Master Assistant preview.
pub const AVAILABLE: bool = false;
pub fn available() -> Vec<crate::plugins::Available> { Vec::new() }
pub fn is_bundled(_: &os_host::Descriptor) -> bool { false }
pub fn enabled(_: &crate::app::State) -> bool { false }
pub fn open(_: &mut crate::app::App, _: bool) {}

pub fn slot(_: &crate::app::State) -> Option<u8> { None }
