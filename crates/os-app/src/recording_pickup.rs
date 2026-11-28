// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Bounded real-time mixer taps. Disk writing stays on the existing recorder threads.
use os_model::{RecordingPoint as Point,InputMonitoring};
use crate::{app::{App,State},mixer::{MCmd,MHit},playlist::{Cmd,PItem,Mark,Popup}};
pub const VISIBILITY:&str="visible_mixer-recording-pickup";
pub use os_session::recording_pickup::{engine, rate, stop, retarget, point, push, delay};
pub fn start(s: &State, tracks: &[u16], secs: f64) {
    let tracks: Vec<_> = tracks.iter().map(|&track| (track, settings(s, track).point)).collect();
    os_session::recording_pickup::start(&tracks, secs);
}
pub fn settings(s:&State,t:u16)->os_model::MixerRecording {crate::mixer::track(s,t).map(|m|m.recording).unwrap_or_default()}
pub fn items(s:&State,t:u16)->Vec<PItem> {
    let now=settings(s,t);let active=s.rec.audio || !s.mixer.recording.live.is_empty();
    let mut v=vec![PItem::head("Record from")];
    for p in Point::ALL {let mut i=PItem::new(p.label(),Cmd::Mixer(MCmd::RecordingPoint(t,p))).mark(Mark::Radio(now.point==p));if active{i=i.off();}v.push(i);}
    v.push(PItem::head("Monitor external input"));
    for m in [InputMonitoring::Off,InputMonitoring::Armed,InputMonitoring::On] {let mut i=PItem::new(m.label(),Cmd::Mixer(MCmd::InputMonitoring(t,m))).mark(Mark::Radio(now.monitor==m));if active{i=i.off();}v.push(i);}
    if now.monitor==InputMonitoring::Off && now.point!=Point::External {v.push(PItem::head("Monitoring off: mixer pickups exclude external input"));}
    v
}
pub fn menu(s:&State,t:u16,x:f32,y:f32)->Popup {Popup{x,y,items:items(s,t),sub:None,scroll:[0;2],cascade:Vec::new()}}
pub fn change(app:&mut App,t:u16,p:Option<Point>,m:Option<InputMonitoring>)->Option<String> {
    if app.audio_rec.is_some(){return Some("Stop recording before changing its pickup or monitoring".into());}
    if crate::mixer::locked(&app.state,t){return Some(crate::mixer::LOCK_MESSAGE.into());}
    let old=settings(&app.state,t);let track=crate::mixer::track_mut(&mut app.state,t)?;
    if let Some(p)=p {track.recording.point=p;}if let Some(m)=m{track.recording.monitor=m;}
    if track.recording!=old {crate::history::record(&mut app.state,"Mixer recording options",crate::history::Snap::MixerRecording(t,old));}
    app.input_monitor.retry();app.invalidate();app.sync_input_monitor().err()
}
pub fn button(ui:&mut crate::app::Ui,s:&State,t:u16,r:os_ui::Rect) {
    if !crate::workspace_visibility::shown(VISIBILITY){return;}
    let p=settings(s,t).point;let hot=ui.hit(r,crate::app::Action::Mix(MHit::RecordingPickup(t)),format!("Record from: {} - click for recording and monitoring options",p.label()));
    ui.c.fill_rrect(r,3.,if hot{ui.t.button_hi}else{ui.t.button});
    ui.c.text_center(os_ui::Font::new(9),r,ui.t.text,p.short());
}

pub fn internal_input()->crate::capture::Input {crate::capture::Input{device:String::new(),asio:false,first:0,stereo:true}}

#[cfg(test)]
mod tests {
    use super::*;
    /// The capture tap `i` (`start` gives the tracks the taps in order): the first is the main capture, the others
    /// the extra ones (`os_session::recording_pickup::source`).
    fn source(i: usize) -> &'static os_session::capture::Capture {
        if i == 0 { os_session::capture::capture() } else { os_session::capture::extra_capture(i - 1) }
    }
    #[test]
    fn recording_pickup_preroll_loop_and_multiple_sources() {
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let mut app=App::new(None);app.state.session.project=crate::locked::Locked::new(crate::app::test_project());
        crate::mixer::track_mut(&mut app.state,2).unwrap().recording.point=Point::Track;
        start(&app.state,&[1,2],1.0);
        assert_eq!(point(1),Some(Point::External));assert_eq!(point(2),Some(Point::Track));
        push(1,44100,44000,0,&vec![0.25;400]);push(2,44100,44000,32,&vec![0.5;400]);
        assert_eq!(source(0).frames(),100);assert_eq!(source(1).frames(),100);
        push(1,44100,0,0,&vec![0.75;200]);push(2,44100,0,32,&vec![1.;200]);
        assert_eq!(source(0).frames(),200,"loop keeps one continuous file");assert_eq!(rate(),44100);assert_eq!(delay(1),32);
        stop();crate::capture::stop_group();
        let n=source(0).frames();push(1,44100,100,0,&vec![1.;200]);assert_eq!(source(0).frames(),n);
        assert_eq!(point(1),None);
    }
    #[test]
    fn recording_pickup_menu_persistence_undo_and_internal_take() {
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        let mut app=App::new(None);app.state.session.project=crate::locked::Locked::new(crate::app::test_project());
        let original=app.state.project.clone();
        assert_eq!(settings(&app.state,1),os_model::MixerRecording::default());
        for point in Point::ALL {assert_eq!(change(&mut app,1,Some(point),None),None);assert_eq!(settings(&app.state,1).point,point);}
        change(&mut app,1,None,Some(InputMonitoring::Off));
        let json=serde_json::to_string(&*app.state.project).unwrap();let loaded:os_model::Project=serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.mixer[1].recording,settings(&app.state,1));
        let old_json=serde_json::to_value(&original.mixer[1]).unwrap();let mut legacy=old_json.clone();legacy.as_object_mut().unwrap().remove("recording");
        let legacy:os_model::MixerTrack=serde_json::from_value(legacy).unwrap();assert_eq!(legacy.recording,Default::default());
        let before=settings(&app.state,1);change(&mut app,1,Some(Point::Input),None);
        crate::history::undo(&mut app.state);assert_eq!(settings(&app.state,1),before);
        crate::history::redo(&mut app.state);assert_eq!(settings(&app.state,1).point,Point::Input);
        crate::settings_file::set_flag(VISIBILITY,false);assert_eq!(items(&app.state,1).len(),12);assert_eq!(settings(&app.state,1).point,Point::Input);
        crate::settings_file::set_flag(VISIBILITY,true);
        assert!(!InputMonitoring::Off.audible(true));assert!(!InputMonitoring::Armed.audible(false));assert!(InputMonitoring::Armed.audible(true));assert!(InputMonitoring::On.audible(false));
        change(&mut app,1,Some(Point::Effects),Some(InputMonitoring::On));
        app.arm_mixer_disk(1);assert!(app.state.mixer.recording.armed.contains(&1));
        app.prepare_mixer_take().unwrap();assert!(engine());assert_eq!(point(1),Some(Point::Effects));
        assert!(change(&mut app,1,Some(Point::External),None).unwrap().contains("Stop recording"));
        struct Feed;
        impl os_engine::engine::MixerInput for Feed {
            fn render(&mut self,_:u32,b:&mut[Vec<f32>]) {b[1].fill(0.25);}
            fn recording_point(&self,t:usize)->Option<Point>{point(t)}
            fn record(&mut self,t:usize,r:u32,s:&os_engine::TransportInfo,d:u32,b:&[f32]){push(t,r,s.frame,d,b);}
        }
        struct Double;
        impl os_engine::instrument::HostedEffect for Double {
            fn process_fx(&mut self,_:u32,l:&mut[f32],r:&mut[f32]) {for v in l.iter_mut().chain(r){*v*=2.;}}
        }
        let mut project=app.state.project.clone();project.channels.clear();project.patterns.clear();project.arrangements.clear();project.mixer[1].slots_enabled=true;
        let mut e=os_engine::Engine::new(&project,&mut os_engine::SampleLibrary::new(None),48000,os_engine::PlayMode::Song);
        e.set_awake(true);e.set_mixer_input(Box::new(Feed));
        e.set_effect(1,0,std::sync::Arc::new(std::sync::Mutex::new(Double)));
        e.process(&mut vec![0.;4800]);
        app.transport.stop();
        for _ in 0..500 {app.one_click_tick();if app.audio_rec.is_none(){break;}std::thread::sleep(std::time::Duration::from_millis(5));}
        assert!(app.audio_rec.is_none());assert!(!engine());
        let channel=app.state.project.channels.last().unwrap();let audio=os_engine::sample::load_shared(std::path::Path::new(channel.sample_path.as_ref().unwrap())).unwrap();
        assert!(audio.frames()>0);assert!(audio.left.iter().all(|v|(*v-0.5).abs()<1e-5),"effect output reaches the actual WAV");
    }
}
