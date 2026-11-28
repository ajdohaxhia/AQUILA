// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! An independent, bounded input queue: monitoring never consumes the disk recorder's samples.
use crate::capture::{Input, Ring};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[derive(Default)]
struct Read {
    owner: Option<u64>,
    last: [f32;2],
    from: [f32;2],
    fade_left: u32,
    fade_len: u32,
    heard: bool,
    starved: bool,
    rate: u32,
    output_rate: u32,
    phase: f64,
    a: Option<[f32; 2]>,
    b: Option<[f32; 2]>,
}
impl Read {
    fn soften(&mut self,rate:u32) {
        if self.heard {self.from=self.last;self.fade_len=(rate/333).max(1);self.fade_left=self.fade_len;}
    }
    fn sample(&mut self,v:Option<[f32;2]>,rate:u32)->[f32;2] {
        let starved=v.is_none();
        if starved!=self.starved {self.soften(rate);self.starved=starved;}
        let mut out=v.unwrap_or([0.;2]);
        if v.is_some() {self.heard=true;}
        if self.fade_left>0 {
            self.fade_left-=1;
            let old=self.fade_left as f32/self.fade_len as f32;
            for k in 0..2 {out[k]=self.from[k]*old+out[k]*(1.-old);}
        }
        self.last=out;out
    }
}
pub struct Monitor {
    pub audible: AtomicBool,
    on: AtomicBool,
    asio: AtomicBool,
    first: AtomicU32,
    stereo: AtomicBool,
    pub track: AtomicU32,
    rate: AtomicU32,
    block: AtomicU32,
    pub dropouts: AtomicU64,
    baseline: AtomicU64,
    pub ring: Ring,
    read: Mutex<Read>,
}
impl Monitor {
    pub fn new() -> Self {
        Self {
            audible:AtomicBool::new(true),
            on: AtomicBool::new(false),
            asio: AtomicBool::new(true),
            first: AtomicU32::new(0),
            stereo: AtomicBool::new(false),
            track: AtomicU32::new(0),
            rate: AtomicU32::new(48000),
            block: AtomicU32::new(0),
            dropouts: AtomicU64::new(0),
            baseline: AtomicU64::new(0),
            ring: Ring::new(1 << 16),
            read: Mutex::new(Read::default()),
        }
    }
    pub fn on(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }
    pub fn start(&self, track: u16, input: &Input) {
        self.on.store(false, Ordering::Release);
        // The UI can wait; neither audio callback nor engine waits for this lock.
        let mut read = self.read.lock().unwrap_or_else(|e| e.into_inner());
        self.ring.clear();
        self.block.store(0,Ordering::Relaxed);
        *read = Read::default();
        self.asio.store(input.asio, Ordering::Relaxed);
        self.first.store(input.first.into(), Ordering::Relaxed);
        self.stereo.store(input.stereo, Ordering::Relaxed);
        self.track.store(track.into(), Ordering::Relaxed);
        self.on.store(true, Ordering::Release);
    }
    pub fn stop(&self) {
        self.on.store(false, Ordering::Release);
    }
    pub fn push(&self, asio: bool, rate: u32, frames: usize, at: impl Fn(usize, usize) -> f32) {
        if !self.on() || self.asio.load(Ordering::Relaxed) != asio {
            return;
        }
        self.rate.store(rate.max(1), Ordering::Relaxed);
        self.block.store(frames.min(u32::MAX as usize) as u32,Ordering::Relaxed);
        let first = self.first.load(Ordering::Relaxed) as usize;
        let second = first + usize::from(self.stereo.load(Ordering::Relaxed));
        self.ring.push_with(frames * 2, |k| {
            let v = at(if k % 2 == 0 { first } else { second }, k / 2);
            if v.is_finite() { v } else { 0.0 }
        });
    }
    #[cfg(any(test, feature = "test-util"))]
    pub fn render(&self, output_rate: u32, tracks: &mut [Vec<f32>]) { self.render_owned(output_rate,tracks,0); }
    /// `render` as the transport calls it, for the suites of a crate built on this one.
    #[cfg(any(test, feature = "test-util"))]
    pub fn render_owned_for_tests(&self, output_rate: u32, tracks: &mut [Vec<f32>], owner: u64) { self.render_owned(output_rate,tracks,owner); }
    fn render_owned(&self, output_rate: u32, tracks: &mut [Vec<f32>], owner:u64) {
        if !self.on() {
            return;
        }
        // Also excludes a retiring engine during a transport transition. Never blocks an audio thread.
        let Ok(mut r) = self.read.try_lock() else {
            return;
        };
        let Some(out) = tracks.get_mut(self.track.load(Ordering::Relaxed) as usize) else {
            return;
        };
        let rate = self.rate.load(Ordering::Relaxed).max(1);
        let handover=r.owner.is_some_and(|old|old!=owner);
        if r.rate != rate || r.output_rate != output_rate {
            *r = Read {
                rate,
                output_rate,
                ..Read::default()
            };
        }
        if handover {r.soften(output_rate);}
        r.owner=Some(owner);
        // A new playback session takes monitoring from the idle/live engine. Discard
        // input accumulated during session construction once, rather than carrying
        // a permanent FIFO delay. Normal callback-sized input bursts remain intact.
        let needed=((out.len()/2) as u64*rate as u64).div_ceil(output_rate.max(1) as u64) as usize;
        let callback=self.block.load(Ordering::Relaxed) as usize;
        // The output feeder may render a whole tick beyond its 6 ms maximum
        // margin. Input must tolerate that scheduling span, even for 16-frame
        // callbacks. This is a ceiling, not a target: normal reads do not wait
        // for this headroom to fill. Keep a separate hysteresis band so slight
        // clock/tick variation cannot trigger repeated sample shaving.
        let headroom = callback.max(needed + (rate as usize * 6).div_ceil(1000));
        let max = needed + if handover { 0 } else { headroom }
            + usize::from(rate != output_rate) * 2;
        let trim_at = max + if handover { 0 } else { headroom };
        if self.ring.waiting_frames() > trim_at {
            if r.heard && !handover { self.dropouts.fetch_add(1, Ordering::Relaxed); }
            while self.ring.waiting_frames() > max {
                self.ring.pop_frame();
            }
            r.soften(output_rate);
            r.a = None;
            r.b = None;
            r.phase = 0.0;
        }
        // The feeder renders whole engine ticks, often larger than a device callback.
        // Keep partial input queued until a complete tick is available. Consuming it
        // now and padding the rest repeats a hole on every small callback.
        let available = self.ring.waiting_frames() + usize::from(r.a.is_some()) + usize::from(r.b.is_some());
        let required = needed + usize::from(rate != output_rate);
        if available < required {
            // Count the start of an interruption, not every silent render tick.
            if r.heard && !r.starved && !handover { self.dropouts.fetch_add(1, Ordering::Relaxed); }
            for f in out.chunks_exact_mut(2) {
                let v = r.sample(None, output_rate);
                f[0] += v[0];
                f[1] += v[1];
            }
            return;
        }
        if rate == output_rate {
            for f in out.chunks_exact_mut(2) {
                let v=r.sample(self.ring.pop_frame(),output_rate);
                f[0] += v[0];
                f[1] += v[1];
            }
            return;
        }
        let step = f64::from(rate) / f64::from(output_rate.max(1));
        for f in out.chunks_exact_mut(2) {
            if r.a.is_none() {
                r.a = self.ring.pop_frame();
            }
            if r.b.is_none() {
                r.b = self.ring.pop_frame();
            }
            let (Some(a), Some(b)) = (r.a, r.b) else {
                let v=r.sample(None,output_rate);f[0]+=v[0];f[1]+=v[1];continue;
            };
            let p = r.phase as f32;
            let v=r.sample(Some([a[0]+(b[0]-a[0])*p,a[1]+(b[1]-a[1])*p]),output_rate);
            f[0]+=v[0];f[1]+=v[1];
            r.phase += step;
            while r.phase >= 1.0 {
                r.phase -= 1.0;
                r.a = r.b;
                r.b = self.ring.pop_frame();
                if r.b.is_none() {
                    r.phase = 0.0;
                    break;
                }
            }
        }
    }
}
pub static EXTRA_MONITORS: [OnceLock<Monitor>; 63] = [const { OnceLock::new() }; 63];
pub fn stop_all() {
    monitor().stop();
    for m in EXTRA_MONITORS.iter().filter_map(OnceLock::get) { m.stop(); }
}
pub fn push_all(asio: bool, rate: u32, frames: usize, at: impl Fn(usize,usize)->f32) {
    monitor().push(asio,rate,frames,&at);
    for m in EXTRA_MONITORS.iter().filter_map(OnceLock::get) { m.push(asio,rate,frames,&at); }
}
pub fn audible(track:usize)->bool {
    std::iter::once(monitor()).chain(EXTRA_MONITORS.iter().filter_map(OnceLock::get))
        .find(|m|m.on() && m.track.load(Ordering::Relaxed) as usize==track).is_none_or(|m|m.audible.load(Ordering::Relaxed))
}
pub fn render_all(rate: u32, tracks: &mut [Vec<f32>], owner:u64) {
    monitor().render_owned(rate,tracks,owner);
    for m in EXTRA_MONITORS.iter().filter_map(OnceLock::get) { m.render_owned(rate,tracks,owner); }
}
/// Monitoring interruptions are separate from device/output underruns.
pub fn visible_dropouts() -> u64 {
    std::iter::once(monitor()).chain(EXTRA_MONITORS.iter().filter_map(OnceLock::get))
        .map(|m| m.dropouts.load(Ordering::Relaxed).saturating_sub(m.baseline.load(Ordering::Relaxed))).sum()
}
pub fn reset_dropouts() {
    for m in std::iter::once(monitor()).chain(EXTRA_MONITORS.iter().filter_map(OnceLock::get)) {
        m.baseline.store(m.dropouts.load(Ordering::Relaxed), Ordering::Relaxed);
    }
}
pub fn monitor() -> &'static Monitor {
    static M: OnceLock<Monitor> = OnceLock::new();
    M.get_or_init(Monitor::new)
}

