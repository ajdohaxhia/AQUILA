// SPDX-License-Identifier: GPL-3.0-or-later
//! ML Patch's graph is the session's (`os_session::ml_patch`): the instances the session hosts carry its `Graph`, so
//! the editor here must name the same type to reach them (`ml_patch::graph`). A copy of the module made a second,
//! unrelated `Graph` that no hosted instance had.
pub use os_session::ml_patch::*;
