// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

pub use os_session::input_monitor::*;
use crate::capture::{Input, InputStream};
use std::sync::{OnceLock, atomic::Ordering};
#[derive(Default)]
pub struct Runtime {
    selected: Option<(u16, Input)>,
    monitored: Vec<(u16, Input)>,
    config: Option<crate::audio_cfg::AudioCfg>,
    error: Option<String>,
    stream: Option<InputStream>,
    pub rate: Option<u32>,
}
impl Runtime {
    pub fn channels(&self) -> Option<u16> { self.stream.as_ref().map(|s|s.channels) }
    pub fn retry(&mut self) {
        self.selected = None;
    }
    fn fail(&mut self, message: String) -> Result<(), String> {
        self.error = Some(message.clone());
        Err(message)
    }
    pub fn owns(&self, input: &Input) -> bool {
        self.stream.is_some() && self.same_device(input)
    }
    pub fn same_device(&self, input: &Input) -> bool {
        self.selected.as_ref().is_some_and(|(_, i)| i.device == input.device && i.asio == input.asio)
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if self.selected.is_some() {
            stop_all();
        }
    }
}
impl crate::app::App {
    pub(crate) fn sync_input_monitor(&mut self) -> Result<(), String> {
        // Headless tests and screenshots must never open a microphone.
        if crate::transport::is_headless() {
            return Ok(());
        }
        for m in std::iter::once(monitor()).chain(EXTRA_MONITORS.iter().filter_map(OnceLock::get)) {
            let t=m.track.load(Ordering::Relaxed) as u16;
            m.audible.store(crate::recording_pickup::settings(&self.state,t).monitor.audible(self.state.mixer.recording.armed.contains(&t)),Ordering::Relaxed);
        }
        let mut all: Vec<_> = self.state.mixer.recording.inputs.iter().filter(|(t,_)| self.state.mixer.recording.armed.contains(t) || crate::recording_pickup::settings(&self.state,**t).monitor==os_model::InputMonitoring::On).map(|(&t,i)|(t,i.clone())).collect();
        all.sort_by_key(|(t,_)|*t);
        let wanted=all.first().cloned();
        let config = crate::audio_cfg::AudioCfg::current();
        if self.input_monitor.selected == wanted && self.input_monitor.monitored == all
            && self.input_monitor.config.as_ref() == Some(&config)
        {
            return self.input_monitor.error.clone().map_or(Ok(()), Err);
        }
        stop_all();
        self.input_monitor.stream = None;
        self.input_monitor.rate = None;
        self.input_monitor.selected = wanted.clone();
        self.input_monitor.monitored = all.clone();
        self.input_monitor.config = Some(config.clone());
        self.input_monitor.error = None;
        if let Some((track, input)) = wanted {
            if input.asio {
                if config.asio_driver() != Some(input.device.as_str()) {
                    return self.input_monitor.fail("Select the input from the ASIO driver currently chosen in Settings > Audio".into());
                }
                if self.transport.opened().as_ref().is_some_and(|d| {
                    d.host == crate::device::ASIO
                        && d.name == input.device
                        && input.first + u16::from(input.stereo) >= d.inputs
                }) {
                    return self
                        .input_monitor
                        .fail("The ASIO driver did not open the selected input channel".into());
                }
            }
            let opened = if input.asio {
                Ok(None)
            } else {
                crate::capture::open_input(&input.device).map(Some)
            };
            let stream = match opened {
                Ok(s) => s,
                Err(e) => {
                    return self
                        .input_monitor
                        .fail(format!("Audio input: {e}. Reselect the input to try again"));
                }
            };
            if stream
                .as_ref()
                .is_some_and(|s| input.first + u16::from(input.stereo) >= s.channels)
            {
                return self
                    .input_monitor
                    .fail("The selected input channel is unavailable".into());
            }
            self.input_monitor.rate = stream.as_ref().map(|s| s.rate);
            self.input_monitor.stream = stream;
            monitor().start(track, &input);
            monitor().audible.store(crate::recording_pickup::settings(&self.state,track).monitor.audible(self.state.mixer.recording.armed.contains(&track)),Ordering::Relaxed);
            let available = self.input_monitor.channels().or_else(|| self.transport.opened().map(|d| d.inputs));
            for (k,(t,i)) in all.iter().skip(1).enumerate() {
                if k >= EXTRA_MONITORS.len() || i.device != input.device || i.asio != input.asio || available.is_some_and(|n| i.first+u16::from(i.stereo)>=n) {
                    stop_all();
                    return self.input_monitor.fail("An armed input is unavailable on the selected interface".into());
                }
                let m=EXTRA_MONITORS[k].get_or_init(Monitor::new);m.start(*t,i);
                m.audible.store(crate::recording_pickup::settings(&self.state,*t).monitor.audible(self.state.mixer.recording.armed.contains(t)),Ordering::Relaxed);
            }
            self.sync_live();
        }
        self.transport
            .listen(self.editors.editor_open() || monitor().on());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(stereo: bool) -> Input {
        Input {
            device: "Synthetic".into(),
            asio: true,
            first: 1,
            stereo,
        }
    }
    #[test]
    fn dropout_counter_excludes_startup_and_counts_recovery_episodes() {
        let m = Monitor::new(); m.start(1, &input(false));
        let mut tracks = vec![vec![0.; 128]; 2];
        m.render(48000, &mut tracks);
        assert_eq!(m.dropouts.load(Ordering::Relaxed), 0);
        m.push(true, 48000, 64, |_,_| 0.2); m.render(48000, &mut tracks);
        for _ in 0..4 { m.render(48000, &mut tracks); }
        assert_eq!(m.dropouts.load(Ordering::Relaxed), 1);
        m.push(true, 48000, 64, |_,_| 0.2); m.render(48000, &mut tracks);
        m.render(48000, &mut tracks);
        assert_eq!(m.dropouts.load(Ordering::Relaxed), 2);
        for _ in 0..80 { m.push(true, 48000, 64, |_,_| 0.3); }
        m.render(48000, &mut tracks);
        assert_eq!(m.dropouts.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn partial_input_waits_for_a_complete_engine_block() {
        let m = Monitor::new(); m.start(1, &input(false));
        let mut tracks = vec![vec![0.; 256 * 2]; 2];
        for batch in 0..4 {
            m.push(true, 48000, 64, |_, f| (batch * 64 + f) as f32);
            m.render(48000, &mut tracks);
            if batch < 3 {
                assert_eq!(m.ring.waiting_frames(), (batch + 1) * 64);
                assert!(tracks[1].iter().all(|&v| v == 0.));
            }
        }
        for f in 0..256 { assert_eq!(tracks[1][2 * f], f as f32); }
        assert_eq!(m.ring.waiting_frames(), 0);
    }

    #[test]
    fn simultaneous_monitor_routes_are_independent() {
        let _lock=crate::capture::TEST_RECORDING.lock().unwrap_or_else(|e|e.into_inner());
        stop_all();
        let input=Input{device:"Synthetic".into(),asio:true,first:0,stereo:false};
        monitor().start(1,&input);
        EXTRA_MONITORS[0].get_or_init(Monitor::new).start(2,&Input{first:1,..input});
        push_all(true,48000,64,|ch,_|if ch==0{0.2}else{-0.4});
        let mut tracks=vec![vec![0.;128];3];
        render_all(48000,&mut tracks,0);
        assert!(tracks[0].iter().all(|&v|v==0.));
        assert!(tracks[1].iter().all(|&v|(v-0.2).abs()<1e-6));
        assert!(tracks[2].iter().all(|&v|(v+0.4).abs()<1e-6));
        stop_all();
    }

    #[test]
    fn selected_input_stereo_mono_resampling_and_disarm() {
        let m = Monitor::new();
        m.start(1, &input(false));
        m.push(true, 48000, 32, |c, _| c as f32 * 0.25);
        let mut tracks = vec![vec![0.; 32]; 3];
        m.render(48000, &mut tracks);
        assert!(tracks[1].iter().all(|v| *v == 0.25));
        assert!(tracks[0].iter().all(|v| *v == 0.));
        m.start(1, &input(true));
        m.push(true, 24000, 20, |c, f| c as f32 * 0.1 + f as f32 * 0.01);
        tracks[1].fill(0.);
        m.render(48000, &mut tracks);
        for (i, v) in tracks[1].chunks_exact(2).enumerate() {
            assert!((v[0] - (0.1 + i as f32 * 0.005)).abs() < 1e-6);
            assert!((v[1] - v[0] - 0.1).abs() < 1e-6);
        }
        m.stop();
        tracks[1].fill(0.);
        m.render(48000, &mut tracks);
        assert!(tracks[1].iter().all(|v| *v == 0.));
    }
    #[test]
    fn monitor_discontinuities_and_starvation_are_declicked_without_queuing_delay() {
        let m=Monitor::new();m.start(1,&input(false));
        let mut tracks=vec![vec![0.;512];2];
        m.push(true,48000,256,|_,_|0.8);m.render_owned_for_tests(48000,&mut tracks,1);
        assert_eq!(tracks[1][510],0.8);
        for _ in 0..20 {m.push(true,48000,64,|_,_|-0.8);}
        tracks[1].fill(0.);m.render_owned_for_tests(48000,&mut tracks,2);
        assert!((tracks[1][0]-0.8).abs()<0.02);
        assert!(tracks[1].chunks_exact(2).map(|f|f[0]).collect::<Vec<_>>().windows(2).all(|w|(w[1]-w[0]).abs()<0.02));
        assert!((tracks[1][510]+0.8).abs()<1e-5);assert_eq!(m.ring.waiting_frames(),0);
        tracks[1].fill(0.);m.render_owned_for_tests(48000,&mut tracks,2);
        assert!((tracks[1][0]+0.8).abs()<0.02);assert_eq!(tracks[1][510],0.);
        m.push(true,48000,256,|_,_|0.8);tracks[1].fill(0.);m.render_owned_for_tests(48000,&mut tracks,2);
        assert!(tracks[1][0].abs()<0.02);assert!((tracks[1][510]-0.8).abs()<1e-5);
    }
    #[test]
    fn continued_monitoring_bounds_backlog_without_truncating_normal_blocks() {
        for (input_block,output_block) in [(64,250),(512,250),(1024,128)] {
            let m=Monitor::new();m.start(1,&input(false));
            let mut tracks=vec![vec![0.;output_block*2];2];
            m.render_owned_for_tests(48000,&mut tracks,1);
            // Stall AFTER ownership is established, as can happen while starting a recording.
            for _ in 0..20 {m.push(true,48000,input_block,|_,_|0.1);}
            m.push(true,48000,input_block,|_,_|0.8);
            m.render_owned_for_tests(48000,&mut tracks,1);
            assert!(m.ring.waiting_frames()<=input_block.max(output_block + 288),"persistent backlog");
        }
        // Ordinary unequal callback sizes preserve the full input, in order.
        let m=Monitor::new();m.start(1,&input(false));
        let mut tracks=vec![vec![0.;128*2];2];
        for batch in 0..4 {
            m.push(true,48000,1024,|_,f|(batch*1024+f) as f32);
            for part in 0..8 {
                tracks[1].fill(0.);m.render_owned_for_tests(48000,&mut tracks,1);
                for f in 0..128 {assert_eq!(tracks[1][2*f],(batch*1024+part*128+f) as f32);}
            }
        }
    }
    #[test]
    fn recording_transition_cannot_leave_a_persistent_monitor_backlog() {
        for rate in [44100,48000,96000] {
            let m=Monitor::new();m.start(1,&input(false));
            let block=64;
            let mut tracks=vec![vec![0.;block*2];2];
            m.render_owned_for_tests(rate,&mut tracks,1); // idle monitoring owns input
            // Input accumulates during a transport start/rebuild, then normal callbacks resume.
            for batch in 0..40 {m.push(true,rate,block,|_,_|batch as f32);}
            m.render_owned_for_tests(rate,&mut tracks,2); // recording session takes over
            assert_eq!(m.ring.waiting_frames(),0);
            for batch in 40..48 {
                m.push(true,rate,block,|_,_|batch as f32);
                tracks[1].fill(0.);m.render_owned_for_tests(rate,&mut tracks,2);
                assert!(tracks[1][0]>=batch as f32-(rate as f32/200./block as f32).ceil(),"stale audio retained");
            }
            // Rendering a different output rate uses input frames for the bound.
            m.push(true,rate,4096,|_,_|1.);
            m.render_owned_for_tests(48000,&mut tracks,3);
            assert!(m.ring.waiting_frames()<=2);
        }
    }
    #[test]
    fn monitoring_does_not_take_samples_from_recording_and_drops_old_backlog() {
        let _lock = crate::capture::TEST_RECORDING
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let m = Monitor::new();
        m.start(1, &input(false));
        let c = crate::capture::capture();
        c.start(0, false);
        c.push(5000, |_, _| 0.2);
        for batch in 0..80 {m.push(true,48000,64,|_,_|if batch<40 {0.1}else{0.4});}
        let mut tracks = vec![vec![0.; 128]; 2];
        m.render(48000, &mut tracks);
        assert!(tracks[1].iter().all(|v| *v == 0.4));
        assert_eq!(c.frames(), 5000);
        c.stop();
    }
}
