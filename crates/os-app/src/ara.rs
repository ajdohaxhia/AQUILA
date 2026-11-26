// SPDX-License-Identifier: GPL-3.0-or-later
//! Clip audio documents and Mixer shortcuts to the same hosted editor.
use crate::{
    app::App,
    playlist::{self as pl, Cmd, PItem, Popup},
};
use os_model::ClipSource;
pub const MARKER: &str = "Lemon ARA";
pub fn is(ch: &os_model::Channel) -> bool {
    ch.plugin.as_deref() == Some(MARKER)
        || ch
            .plugin_state
            .as_deref()
            .is_some_and(os_vst3::ara::State::is)
}
pub const MIXER_LINK: &str = "ML ARA Clip";
pub const MIXER_LINK_VISIBILITY: &str = "visible_ara_mixer_link";

pub fn link_source(slot: &os_model::SlotPlugin) -> Option<u16> {
    if slot.name != MIXER_LINK { return None; }
    Some(u16::from_le_bytes(slot.state.get(..2)?.try_into().ok()?))
}

pub fn link_title(slot: &os_model::SlotPlugin) -> Option<String> {
    link_source(slot)?;
    let title = std::str::from_utf8(slot.state.get(2..)?).ok()?;
    Some(format!("ARA · {title}"))
}

fn link_slot(app: &App, track: u16, source: u16) -> Option<u8> {
    use crate::mixer as m;
    if !crate::workspace_visibility::shown(MIXER_LINK_VISIBILITY) { return None; }
    let slots = &m::track(&app.state, track)?.slots;
    (0..m::user_slot_capacity(&app.state, track)).map(|i| m::visible_user_slot(&app.state, track, i))
        .find(|&k| slots[k as usize].as_ref().and_then(link_source) == Some(source))
        .or_else(|| (!m::locked(&app.state, track)).then(|| {
            (0..m::user_slot_capacity(&app.state, track)).map(|i| m::visible_user_slot(&app.state, track, i))
                .find(|&k| slots[k as usize].is_none())
        }).flatten())
}

fn add_link(app: &mut App, track: u16, slot: u8, source: u16, title: &str) {
    let mut state = source.to_le_bytes().to_vec();
    state.extend_from_slice(title.as_bytes());
    if let Some(mixer) = crate::mixer::track_mut(&mut app.state, track) {
        mixer.slots[usize::from(slot)] = Some(os_model::SlotPlugin { id: os_model::Id::NONE, name: MIXER_LINK.into(), state });
    }
}

pub fn open_link(app: &mut App, source: u16) -> Option<String> {
    let Some(row) = app.state.project.channels.iter().position(|c| c.index == source && is(c)) else {
        return Some("This clip no longer has an ARA document.".into());
    };
    if !app.plugins.hosted.get(&row).is_some_and(|h| h.ara_saved.is_some()) {
        if let Err(e) = restore(app, row) { return Some(format!("ARA: {e}")); }
    }
    app.state.pending_open = Some(row);
    app.invalidate();
    None
}

fn row(app: &App, i: usize) -> Option<usize> {
    let ClipSource::Channel(id) = pl::clips(&app.state).get(i)?.source else {
        return None;
    };
    app.state
        .project
        .channels
        .iter()
        .position(|c| c.index == id && c.kind == os_model::ChannelKind::AudioClip)
}
pub(crate) fn choices(app: &App) -> Vec<os_host::Descriptor> {
    let mut out: Vec<_> = app
        .plugins
        .effects
        .iter()
        .chain(app.plugins.available.iter())
        .filter(|a| {
            a.desc.format == os_host::Format::Vst3 && a.desc.features.iter().any(|f| f == "ARA")
        })
        .map(|a| a.desc.clone())
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    out.dedup_by(|a, b| a.uid() == b.uid());
    out
}
const OPEN_EXISTING: usize = usize::MAX;
const OPEN_INDEPENDENT: usize = usize::MAX - 1;

fn current_document(project: &os_model::Project, clip: &os_model::Clip, channel: &os_model::Channel) -> bool {
    let Some(arrangement) = project.arrangement() else { return false; };
    let mut occurrences = project.arrangements.iter().flat_map(|a| {
        a.clips.iter().filter(move |c| c.source == ClipSource::Channel(channel.index))
            .map(move |c| (a.number, c))
    });
    let Some((first_arrangement, first_clip)) = occurrences.next() else { return false; };
    let Some(second) = occurrences.next() else { return true; };
    let owner = channel.plugin_state.as_deref().and_then(os_vst3::ara::State::owner);
    if let Some(owner) = owner {
        let occurrence = (arrangement.number, clip.id());
        if owner == occurrence { return true; }
        if std::iter::once((first_arrangement, first_clip)).chain(std::iter::once(second)).chain(occurrences)
            .any(|(number, other)| (number, other.id()) == owner) {
            return false;
        }
    }
    // A pre-existing shared document stays with its first occurrence.
    first_arrangement == arrangement.number && std::ptr::eq(first_clip, clip)
}

pub fn menu(app: &mut App, i: usize) -> Option<String> {
    let r = row(app, i)?;
    let channel = &app.state.project.channels[r];
    let attached = is(channel);
    let owns_document = attached && current_document(&app.state.project, &pl::clips(&app.state)[i], channel);
    let mut items = vec![PItem::head("ARA audio editor")];
    if owns_document {
        items.push(PItem::new(
            "Open current editor",
            Cmd::AraChoose(i, OPEN_EXISTING),
        ));
        items.push(PItem::new(
            "Remove ARA edits (use original audio)",
            Cmd::AraRemove(i),
        ));
        items.push(PItem::sep());
    }
    if attached && !owns_document {
        items.push(PItem::new("Open independent editor", Cmd::AraChoose(i, OPEN_INDEPENDENT)));
    }
    let editors = choices(app);
    let fresh = editors.iter().enumerate()
        .map(|(k, d)| PItem::new(d.name.clone(), Cmd::AraChoose(i, k))).collect::<Vec<_>>();
    if attached {
        if !fresh.is_empty() { items.push(PItem::sub("Start independent ARA editing", fresh)); }
    } else {
        items.extend(fresh);
    }
    if editors.is_empty() && !attached {
        items.push(PItem::head("No ARA editors found in the plugin list"));
    }
    items.push(PItem::sep());
    items.push(PItem::new("Scan for ARA editors...", Cmd::AraScan));
    app.state.playlist.popup = Some(Popup {
        x: app.mouse.0,
        y: app.mouse.1,
        items,
        sub: None,
        scroll: [0; 2],
        cascade: Vec::new(),
    });
    None
}
pub fn choose(app: &mut App, i: usize, k: usize) -> Option<String> {
    let r = row(app, i)?;
    if k == OPEN_EXISTING && current_document(&app.state.project, &pl::clips(&app.state)[i], &app.state.project.channels[r]) {
        let channel = &app.state.project.channels[r];
        let (track, source) = (channel.mixer_track, channel.index);
        if let Some(slot) = link_slot(app, track, source) {
            if crate::mixer::track(&app.state, track).is_some_and(|m| m.slots[usize::from(slot)].is_none()) {
                if let Some(m) = crate::mixer::channel_copy(&app.state, track) {
                    let title = channel.name.clone().unwrap_or_else(|| "Audio Clip".into());
                    crate::history::record(&mut app.state, "Add ARA Mixer link", crate::history::Snap::MixerChannel(Box::new(m)));
                    add_link(app, track, slot, source, &title);
                    app.state.piano.dirty = true;
                }
            }
        }
        return open_link(app, source);
    }
    let ch = &app.state.session.project.channels[r];
    let (plugin_path, class_id) = if k == OPEN_INDEPENDENT || k == OPEN_EXISTING {
        let saved = match os_vst3::ara::State::decode(ch.plugin_state.as_deref()?) {
            Ok(saved) => saved,
            Err(e) => return Some(format!("ARA: {e}")),
        };
        (saved.plugin_path, saved.class_id)
    } else {
        let editor = choices(app).get(k)?.clone();
        (editor.path, editor.id)
    };
    let source = ch.sample_path.clone()?;
    // The document owns unprocessed source audio. Sampler time/pitch transformations are not silently discarded.
    if ch.sound.fx != os_model::SampleFx::default()
        || ch.sound.start_offset() != 0.
        || ch.envelopes.iter().any(|e| e.enabled)
    {
        return Some("Render this clip's sampler processing to audio first, then open the rendered clip with ARA".into());
    }
    if (ch.misc.main_pitch && app.state.project.main_pitch.unwrap_or(0) != 0)
        || ch.pitch_cents != 0
        || ch.misc.fine != 0
        || ch.misc.root != 60
        || ch.sound.stretch_time().is_some()
        || (ch.sound.stretch_multiplier() - 1.).abs() > 1e-6
        || ch.sound.stretch_cents() != 0.
    {
        return Some("Reset this clip's time stretch and pitch first, then edit its timing and pitch in the ARA editor".into());
    }
    let (id, track) = (ch.index, ch.mixer_track);
    let independent = is(ch) || app.state.project.arrangements.iter().flat_map(|a| &a.clips)
        .filter(|clip| clip.source == ClipSource::Channel(id)).take(2).count() > 1;
    let target_row = if independent { app.state.project.channels.len() } else { r };
    if independent && app.state.project.channels.iter().any(|c| c.index == u16::MAX) {
        return Some("There is no free audio channel for an independent ARA document.".into());
    }
    let slot = link_slot(app, track, if independent { u16::MAX } else { id });
    let mut undo = Vec::new();
    if let Some(m) = slot.and_then(|_| crate::mixer::channel_copy(&app.state, track)) {
        undo.push(crate::history::Snap::MixerChannel(Box::new(m)));
    }
    if independent {
        undo.push(crate::history::Snap::Tail(Box::new(crate::history::Tail::now(&app.state, target_row))));
        undo.push(crate::history::Snap::Vocals(app.state.project.vocal_edits.clone()));
    } else if let Some(before) = crate::history::Snap::ara(&app.state, r) {
        undo.push(before);
    }
    let arrangement = app.state.project.arrangement()?;
    let existing = arrangement.clips[i].id();
    let clip_id = if existing > 0 && arrangement.clips.iter().filter(|c| c.id() == existing).take(2).count() == 1 {
        existing
    } else {
        let Some(next) = arrangement.clips.iter().map(os_model::Clip::id).max().unwrap_or(0).checked_add(1) else {
            return Some("There is no free Playlist clip ID for this ARA document.".into());
        };
        next
    };
    let owner = (arrangement.number, clip_id);
    match app.state.session.plugins.create_ara(target_row, &plugin_path, &class_id, &source, None, Some(owner)) {
        Err(e) => Some(format!("ARA: {e}")),
        Ok(name) => {
            pl::snapshot_with(&mut app.state, "Add ARA editor", crate::history::Snap::Several(undo));
            let id = if independent {
                pl::make_unique(&mut app.state, i).expect("validated audio clip")
            } else { id };
            pl::arr_mut(&mut app.state).clips[i].set_id(clip_id);
            let r = target_row;
            let blob = app.plugins.hosted[&r].ara_saved.clone().unwrap();
            let title = app.state.project.channels[r].name.clone().unwrap_or_else(|| "Audio Clip".into());
            if let Some(slot) = slot {
                add_link(app, track, slot, id, &format!("{name} · {title}"));
            }
            let ch = &mut app.state.project.channels[r];
            ch.plugin = Some(MARKER.into());
            ch.plugin_state = Some(blob);
            app.sampler_failed.retain(|&x| x != r);
            if !app.state.hosted_rows.contains(&r) {
                app.state.hosted_rows.push(r);
            }
            app.state.session.plugins.added(&app.state.session.settings, r);
            app.transport.set_instruments(app.plugins.instruments());
            app.state.piano.dirty = true;
            app.state.pending_open = Some(r);
            app.invalidate();
            Some(if slot.is_some() {
                format!("Opened {name} for this clip. Reopen it from the Mixer insert.")
            } else if crate::workspace_visibility::shown(MIXER_LINK_VISIBILITY) {
                format!("Opened {name} for this clip. Free an unlocked Mixer slot to add a shortcut.")
            } else {
                format!("Opened {name} for this clip.")
            })
        }
    }
}
pub fn remove(app: &mut App, i: usize) -> Option<String> {
    let r = row(app, i)?;
    if !is(&app.state.project.channels[r]) {
        return None;
    }
    if let Err(e) = store(app) {
        return Some(e);
    }
    let id = app.state.project.channels[r].index;
    let mut undo = vec![crate::history::Snap::ara(&app.state, r)?];
    undo.extend(app.state.project.mixer.iter()
        .filter(|m| m.slots.iter().flatten().any(|slot| link_source(slot) == Some(id)))
        .filter_map(|m| crate::mixer::channel_copy(&app.state, m.index))
        .map(|m| crate::history::Snap::MixerChannel(Box::new(m))));
    crate::history::record(&mut app.state, "Remove ARA editor", crate::history::Snap::Several(undo));
    for m in &mut app.state.project.mixer {
        for slot in &mut m.slots {
            if slot.as_ref().and_then(link_source) == Some(id) { *slot = None; }
        }
    }
    let ch = &mut app.state.project.channels[r];
    ch.plugin = None;
    ch.plugin_state = None;
    reconcile(app);
    app.invalidate();
    Some("ARA removed; original audio restored. Undo restores the edits.".into())
}
pub fn restore(app: &mut App, r: usize) -> Result<(), String> {
    let ch = &app.state.session.project.channels[r];
    let saved =
        os_vst3::ara::State::decode(ch.plugin_state.as_deref().ok_or("Missing ARA archive")?)?;
    let source = ch
        .sample_path
        .as_deref()
        .ok_or("ARA source audio is missing")?;
    app.state.session.plugins.create_ara(
        r,
        &saved.plugin_path,
        &saved.class_id,
        source,
        ch.plugin_state.as_deref(),
        None,
    )?;
    if !app.state.hosted_rows.contains(&r) {
        app.state.hosted_rows.push(r);
    }
    app.sampler_failed.retain(|&x| x != r);
    app.transport.set_instruments(app.plugins.instruments());
    Ok(())
}
/// Archive changes on the UI thread, outside the audio callback. Archive failure never clears existing edits.
pub fn edited(app: &mut App, r: usize, blob: Option<Vec<u8>>) {
    if !app.state.project.channels.get(r).is_some_and(is) {
        return;
    }
    if let Some(h) = app.plugins.hosted.get_mut(&r) {
        h.ara_dirty = true;
    }
    let Some(blob) = blob else {
        app.state.message =
            Some("ARA edits could not be archived. Keep the editor open and retry saving.".into());
        return;
    };
    if Some(&blob) == app.state.project.channels[r].plugin_state.as_ref() {
        if let Some(h) = app.plugins.hosted.get_mut(&r) {
            h.ara_dirty = false;
        }
        return;
    }
    if let Some(before) = crate::history::Snap::ara(&app.state, r) {
        crate::history::record(&mut app.state, "ARA edit", before);
    }
    app.state.project.channels[r].plugin_state = Some(blob.clone());
    if let Some(h) = app.plugins.hosted.get_mut(&r) {
        h.ara_saved = Some(blob);
        h.ara_dirty = false;
    }
}
pub fn store(app: &mut App) -> Result<(), String> {
    // A save immediately after Undo must not overwrite its archive with the old running document.
    reconcile(app);
    let rows: Vec<_> = app
        .plugins
        .hosted
        .iter()
        .filter(|(_, h)| h.ara_saved.is_some())
        .map(|(&r, _)| r)
        .collect();
    let mut blobs = Vec::new();
    for r in rows {
        blobs.push((
            r,
            app.plugins.state(r).ok_or(
                "An ARA editor could not save its edits. The project was not overwritten.",
            )?,
        ));
    }
    for (r, blob) in blobs {
        if let Some(ch) = app.state.project.channels.get_mut(r).filter(|c| is(c)) {
            ch.plugin_state = Some(blob.clone());
        }
        if let Some(h) = app.plugins.hosted.get_mut(&r) {
            h.ara_saved = Some(blob);
            h.ara_dirty = false;
        }
    }
    Ok(())
}
/// History restores model archives; replace only affected instances. No model mutation while processing.
pub fn reconcile(app: &mut App) {
    let rows: Vec<_> = app
        .plugins
        .hosted
        .iter()
        .filter_map(|(&r, h)| {
            let Some(ch) = app.state.project.channels.get(r) else {
                return h.ara_saved.as_ref().map(|_| r);
            };
            let needs = match (&h.ara_saved, is(ch)) {
                (Some(st), true) => {
                    Some(st) != ch.plugin_state.as_ref()
                        || ch
                            .sample_path
                            .as_deref()
                            .map(crate::plugins::resolve_sample)
                            != h.ara_source
                }
                (Some(_), false) | (None, true) => true,
                _ => false,
            };
            needs.then_some(r)
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    for r in rows {
        app.plugins.drop_key(r);
        app.state.hosted_rows.retain(|&x| x != r);
        app.sampler_failed.retain(|&x| x != r);
    }
    app.transport.set_instruments(app.plugins.instruments());
    app.state.piano.dirty = true;
}

/// A source document may appear more than once: use the selected occurrence,
/// then the playing occurrence, then the first one. Never route requests to another channel.
pub fn playback_request(app:&mut App,row:usize,request:os_host::PlaybackRequest) {
    use os_host::PlaybackRequest as R;
    let Some(channel)=app.state.project.channels.get(row).filter(|c|is(c)) else {return};
    let matches=|c:&&os_model::Clip| c.source==ClipSource::Channel(channel.index)&&!c.muted;
    let all=pl::clips(&app.state);
    let at=if app.transport.playing()&&app.transport.song(){app.transport.tick().max(0.) as u32}else{app.state.playlist.pos};
    let selected=app.state.playlist.sel.iter().filter_map(|&i|all.get(i)).find(matches);
    let Some(clip)=selected.or_else(||all.iter().filter(matches).find(|c|c.position<=at&&at<c.end())).or_else(||all.iter().find(matches)).cloned() else {return};
    let tempo=app.state.project.tick_tempo(app.state.tempo);
    let tick=|seconds| source_tick(&clip,seconds,app.state.project.ppq,tempo);
    // Resolve before mutating app to avoid holding references across a restart.
    let seek=if let R::Seek(seconds)=request {tick(seconds)} else {None};
    let cycle=if let R::Cycle(Some((a,b)))=request {tick(a).zip(tick(b)).filter(|(a,b)|b>a)}else{None};
    match request {
        R::Start=>{
            app.state.song_mode=true;
            if !app.transport.playing()||!app.transport.song(){
                let pos=app.state.playlist.pos.clamp(clip.position,clip.end().saturating_sub(1));
                app.play_song_from(pos);
            }else if app.transport.paused(){app.transport.resume();}
        }
        R::Stop=>app.act(crate::app::Action::Stop),
        R::Seek(_)=>if let Some(t)=seek {app.state.song_mode=true;app.set_song_pos(t,true);},
        R::Cycle(range)=>{
            if range.is_none()||cycle.is_some(){
                app.state.song_mode=true;app.state.playlist.range=cycle;
                if app.transport.playing()&&!app.transport.paused(){app.play_song_from(cycle.map_or(at,|r|r.0));}
            }
        }
    }
    app.invalidate();
}
fn source_tick(clip:&os_model::Clip,seconds:f64,ppq:u16,tempo:f64)->Option<u32> {
    if !seconds.is_finite()||!tempo.is_finite()||tempo<=0. {return None;}
    let start=clip.window.map_or(0.,|w|w.0.max(0.)/1000.);
    let position=f64::from(clip.position)+(seconds-start)*tempo/60.*f64::from(ppq.max(1));
    Some(position.round().clamp(f64::from(clip.position),f64::from(clip.end())) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ara_mixer_link_is_saved_and_follows_channel_identity() {
        let mut app = App::new(None);
        app.state.session.project = crate::locked::Locked::new(crate::app::test_project());
        let source = app.state.project.channels[1].index;
        let slot = link_slot(&app, 0, source).unwrap();
        add_link(&mut app, 0, slot, source, "Editor · Clip");
        assert_eq!(link_slot(&app, 0, source), Some(slot));
        pl::arr_mut(&mut app.state);
        let mut file = os_flp::project::create(&app.state.project).unwrap();
        os_flp::project::apply(&app.state.project, &mut file).unwrap();
        let mut project = os_flp::project::load(&file).unwrap();
        assert_eq!(link_title(project.mixer[0].slots[slot as usize].as_ref().unwrap()).as_deref(), Some("ARA · Editor · Clip"));
        let mut map = (0..project.channels.len()).map(Some).collect::<Vec<_>>();
        map.swap(0, 1);
        crate::rack::remap_project(&mut project, &map);
        assert_eq!(link_source(project.mixer[0].slots[slot as usize].as_ref().unwrap()), Some(0));
    }

    #[test]
    fn an_earlier_copy_does_not_take_ownership_of_the_original_document() {
        let mut app = App::new(None);
        app.state.session.project = crate::locked::Locked::new(crate::app::test_project());
        let channel = &mut app.state.project.channels[0];
        channel.kind = os_model::ChannelKind::AudioClip;
        channel.plugin = Some(MARKER.into());
        channel.plugin_state = Some(os_vst3::ara::State {
            plugin_path: "Editor.vst3".into(),
            class_id: "editor".into(),
            source_path: "audio.wav".into(),
            factory_id: "factory".into(),
            archive_id: "archive".into(),
            frames: 8,
            rate: 48_000,
            fingerprint: 0,
            owner: Some((0, 42)),
            archive: vec![1, 2, 3],
        }.encode().unwrap());
        let mut original = os_model::Clip::new(192, 96, 0, ClipSource::Channel(0));
        original.set_id(42);
        let copy = os_model::Clip::new(0, 96, 0, ClipSource::Channel(0));
        pl::arr_mut(&mut app.state).clips = vec![copy, original];
        let mut file = os_flp::project::create(&app.state.project).unwrap();
        os_flp::project::apply(&app.state.project, &mut file).unwrap();
        let project = os_flp::project::load(&file).unwrap();
        let clips = &project.arrangement().unwrap().clips;
        let channel = &project.channels[0];
        assert!(!current_document(&project, &clips[0], channel));
        assert!(current_document(&project, &clips[1], channel));
        app.state.session.project = crate::locked::Locked::new(project);
        menu(&mut app, 0);
        assert!(app.state.playlist.popup.as_ref().unwrap().items.iter()
            .any(|item| item.cmd == Cmd::AraChoose(0, OPEN_INDEPENDENT)));
    }

    #[test]
    fn playback_fix_ara_seek_maps_trimmed_source_to_occurrence() {
        let mut clip=os_model::Clip::new(960,384,2,ClipSource::Channel(0));
        clip.window=Some((2000.,4000.));
        assert_eq!(source_tick(&clip,2.,96,90.),Some(960));
        assert_eq!(source_tick(&clip,3.,96,90.),Some(1104));
        assert_eq!(source_tick(&clip,-10.,96,90.),Some(960));
        assert_eq!(source_tick(&clip,100.,96,90.),Some(1344));
        assert_eq!(source_tick(&clip,f64::NAN,96,90.),None);
    }
}
