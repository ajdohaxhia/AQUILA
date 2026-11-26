// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! The MIDI Out channel's plugin, which sends its notes out of a MIDI port (spec: plugins/midi-out.md). The session
//! owns it (`os_session::lemon_midi_out`): the interface reaches it through this module.

pub use os_session::lemon_midi_out::*;
