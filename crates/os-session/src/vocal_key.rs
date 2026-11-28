// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Stable key for an original-source-relative vocal render.
use sha2::{Digest, Sha256};

pub fn key(d: &os_model::vocal::Edit) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(&(
        "clip-edits-v3", d.enabled, &d.source, &d.source_hash, d.rate, d.frames,
        &d.notes, &d.curve, &d.timing, d.strength, d.formant,
    )).unwrap_or_default()))
}
