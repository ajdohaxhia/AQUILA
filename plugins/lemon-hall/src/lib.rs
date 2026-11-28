// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

pub use lemon_small_fx::hall::LemonHall;
#[cfg(not(feature = "linked"))]
use nih_plug::prelude::*;
#[cfg(not(feature = "linked"))]
nih_export_clap!(LemonHall);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(LemonHall);
os_plugkit::linked_clap_entry!(LemonHall);
