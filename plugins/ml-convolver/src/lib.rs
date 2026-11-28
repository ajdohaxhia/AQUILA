// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

#![allow(semicolon_in_expressions_from_non_local_macros)]
#[cfg(not(feature = "linked"))]
use nih_plug::prelude::*;
pub type Effect = ml_space_core::convolver::Convolver;
#[cfg(not(feature = "linked"))]
nih_export_clap!(Effect);
#[cfg(not(feature = "linked"))]
nih_export_vst3!(Effect);
os_plugkit::linked_clap_entry!(Effect);
