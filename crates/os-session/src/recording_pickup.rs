// Copyright (C) 2026 Michael Lane
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

use std::sync::atomic::{AtomicBool,AtomicU32,AtomicU64,AtomicUsize,Ordering};
use os_model::RecordingPoint as Point;
static WRITING:AtomicBool=AtomicBool::new(false);
static ENGINE:AtomicBool=AtomicBool::new(false);
static START:AtomicU64=AtomicU64::new(0);
static RATE:AtomicU32=AtomicU32::new(48000);
struct Tap { track:AtomicUsize, point:AtomicUsize, started:AtomicBool, delay:AtomicU32 }
static TAPS:[Tap;64]=[const {Tap{track:AtomicUsize::new(usize::MAX),point:AtomicUsize::new(0),started:AtomicBool::new(false),delay:AtomicU32::new(0)}};64];
pub fn engine()->bool {ENGINE.load(Ordering::Acquire)}
pub fn rate()->u32 {RATE.load(Ordering::Relaxed)}
fn source(k:usize)-> &'static crate::capture::Capture {if k==0 {crate::capture::capture()} else {crate::capture::extra_capture(k-1)}}
pub fn start(tracks: &[(u16, Point)], secs: f64) {
    stop();
    START.store(secs.to_bits(), Ordering::Relaxed);
    for tap in &TAPS {
        tap.track.store(usize::MAX, Ordering::Relaxed);
        tap.started.store(false, Ordering::Relaxed);
        tap.delay.store(0, Ordering::Relaxed);
    }
    for (index, &(track, point)) in tracks.iter().enumerate().take(TAPS.len()) {
        let tap = &TAPS[index];
        tap.track.store(track as usize, Ordering::Relaxed);
        tap.point.store(point as usize, Ordering::Relaxed);
        source(index).start(0, true);
    }
    ENGINE.store(true, Ordering::Release);
}

pub fn stop(){ENGINE.store(false,Ordering::Release);while WRITING.load(Ordering::Acquire){std::thread::yield_now();}}
pub fn retarget(secs:f64){START.store(secs.to_bits(),Ordering::Release);}
pub fn point(track:usize)->Option<Point> {
    if !engine(){return None;} TAPS.iter().find(|t|t.track.load(Ordering::Relaxed)==track).map(|t|Point::ALL[t.point.load(Ordering::Relaxed)])
}
pub fn push(track:usize,rate:u32,frame:u64,delay:u32,audio:&[f32]) {
    if !engine() || WRITING.compare_exchange(false,true,Ordering::Acquire,Ordering::Relaxed).is_err(){return;}
    struct Unlock;impl Drop for Unlock {fn drop(&mut self){WRITING.store(false,Ordering::Release);}}let _unlock=Unlock;
    if !engine(){return;} let Some(k)=TAPS.iter().position(|t|t.track.load(Ordering::Relaxed)==track) else{return;};
    let t=&TAPS[k];let frames=audio.len()/2;let start=(f64::from_bits(START.load(Ordering::Relaxed))*rate as f64).round() as u64;
    let skip=if t.started.load(Ordering::Relaxed){0}else{start.saturating_sub(frame).min(frames as u64) as usize};
    if skip==frames{return;} t.started.store(true,Ordering::Relaxed);t.delay.store(delay,Ordering::Relaxed);RATE.store(rate,Ordering::Relaxed);
    source(k).push(frames-skip,|c,f|audio[(f+skip)*2+c]);
}
pub fn delay(k:usize)->u32 {TAPS[k].delay.load(Ordering::Relaxed)}
