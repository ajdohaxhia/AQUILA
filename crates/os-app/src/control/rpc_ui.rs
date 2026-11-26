// Copyright (C) 2026 thacer13
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! The interface's side of `crate::control::rpc` (step 9: the session's side stays there).

#![allow(unused_imports)]
use serde_json::{json, Value};
use super::{By, Command, Level};
use crate::app::App;
use crate::session::Session;
#[allow(unused_imports)]
use crate::control::rpc::*;

impl App {
    /// Answers one request line from client `c` (`None`: a notification, no answer): on the session, with the views
    /// following and what the user is to decide asked.
    pub fn rpc_line(&mut self, c: &mut Client, line: &str) -> Option<String> {
        let selected = self.state.selected_channel;
        let mut asks = Asks { interface: true, ..Default::default() };
        let out = self.host(|s| rpc_line_in(s, c, line, selected, &mut asks));
        let history = self.asked(asks);
        match (out, history) {
            (Some(line), Some(result)) => {
                let mut reply: Value = serde_json::from_str(&line).expect("RPC reply is JSON");
                if reply.get("result").is_some() {
                    reply["result"] = result;
                }
                Some(reply.to_string())
            }
            (line, _) => line,
        }
    }

    /// Method `m` with `params` for client `c`, as `rpc_line` answers it.
    #[cfg_attr(not(test), allow(dead_code))] // used by tests
    pub fn rpc(&mut self, c: &mut Client, m: &str, params: &Value) -> Result<Value, Fail> {
        let selected = self.state.selected_channel;
        let mut asks = Asks { interface: true, ..Default::default() };
        let out = self.host(|s| rpc_in(s, c, m, params, selected, &mut asks));
        let history = self.asked(asks);
        out.map(|result| history.unwrap_or(result))
    }

    /// What a request left to the user is asked.
    fn asked(&mut self, asks: Asks) -> Option<Value> {
        let history = asks.history.map(|(undo, by)| {
            let name = if undo { crate::history::undo(&mut self.state) } else { crate::history::redo(&mut self.state) };
            if let Some(name) = &name {
                let ses = &mut self.state.session;
                let tempo = ses.tempo();
                ses.control.saw(&ses.project, tempo);
                ses.control.event(&by, if undo { "undo" } else { "redo" }, name.clone(), Vec::new());
            }
            json!({ "step": name })
        });
        if let Some(n) = asks.proposal {
            self.ask_proposal(n);
        }
        if let Some((who, path)) = asks.save {
            self.ask_save(&who, path);
        }
        history
    }
}

impl App {
    /// Asks the user whether client `who` may save the project to `path`.
    pub fn ask_save(&mut self, who: &str, path: std::path::PathBuf) {
        use crate::dialog::{Answer, Kind, Msg, Then};
        let text = vec![(who.to_string(), true), (" asks to save the project to ".into(), false), (path.display().to_string(), true), (".".into(), false)];
        let mut msg = Msg::new(Kind::Warning, "Save", text, &[Answer::Yes, Answer::No], Then::ClientSave(path));
        msg.labels = vec!["Save", "Don't save"];
        self.show_msg(msg);
    }

    /// Asks the user about proposal `n`: what it would change, Apply or Discard (spec: roadmap/control-plane.md
    /// "Proposals").
    pub fn ask_proposal(&mut self, n: u64) {
        use crate::dialog::{Answer, Kind, Msg, Then};
        let Some(p) = self.state.session.control.proposals.iter().find(|p| p.n == n) else { return };
        let who = match &p.by {
            super::By::Client(c) => c.clone(),
            super::By::User => "You".into(),
        };
        const SHOWN: usize = 8;
        let mut lines: Vec<String> = p.changes.iter().take(SHOWN).map(|c| format!("\n  {}: {} > {}", c.path, c.before, c.after)).collect();
        if p.changes.len() > SHOWN {
            lines.push(format!("\n  and {} more", p.changes.len() - SHOWN));
        }
        let head = format!(" proposes \"{}\" ({} change{}):", p.name, p.changes.len(), if p.changes.len() == 1 { "" } else { "s" });
        let text = vec![(who, true), (head, false), (lines.concat(), false)];
        let mut msg = Msg::new(Kind::Info, "Proposal", text, &[Answer::Yes, Answer::No], Then::Proposal(n));
        msg.labels = vec!["Apply", "Discard"];
        self.show_msg(msg);
    }
}
