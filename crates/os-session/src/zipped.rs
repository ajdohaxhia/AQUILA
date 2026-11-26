// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Zipped projects: a project file and its samples in one .zip, as FL's zipped loop packages are (the project file and
//! the samples side by side at the top of the zip). Opening a .zip unzips it into a folder beside it, then opens its
//! project file: the samples are found by name in the project's folder (`crate::missing`). FILE > Export zipped
//! project... writes the project and every sample it uses into a .zip. Both run on a thread of their own behind a
//! progress panel.
//!
// spec: ui/zipped-projects.md

#![allow(deprecated)] // writes the project directly (`crate::locked`; control-plane step 14 removes this)
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;


/// Where a zip is unzipped when its own folder cannot be written to (a read-only drive, say): this folder in the user
/// folder.
pub const UNZIP_DIR: &str = "Unzipped projects";
/// Files already compressed are stored as they are.
const STORED: &[&str] = &["mp3", "ogg", "flac", "m4a", "aac", "opus", "rx2", "rex", "rcy", "zip"];

/// A zip being unzipped or written: the panel's title and file, and the bytes done of the total.
pub struct Zipping {
    pub rx: Receiver<Result<Done, String>>,
    pub title: &'static str,
    pub name: String,
    pub done: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
}

pub enum Done {
    /// The project file unzipped.
    Unzipped { project: PathBuf, archive: PathBuf },
    /// The zip written, how many samples it holds, and the samples not found (not in it).
    Zipped { zip: PathBuf, samples: usize, missing: Vec<String> },
}

/// The file name of a stored sample path (either slash).
pub fn file_name(p: &str) -> &str {
    p.rsplit(['\\', '/']).next().unwrap_or(p)
}

fn is_project(p: &Path) -> bool {
    crate::project_files::is_project(p)
}

// --- Opening a zip ---

/// A file in the zip: its index, its path inside the zip, its size.
type Entry = (usize, PathBuf, u64);

/// Unzips `zip` into a folder beside it named after it (`unzip_into`), else under `fallback`; returns the project file:
/// the one nearest the top of the zip (then by name). Names that would leave the folder are skipped.
pub fn unzip(zip: &Path, fallback: Option<&Path>, done: &AtomicU64, total: &AtomicU64) -> Result<PathBuf, String> {
    let mut a = zip::ZipArchive::new(io::BufReader::new(File::open(zip).map_err(|e| e.to_string())?)).map_err(|e| e.to_string())?;
    let mut entries: Vec<Entry> = Vec::new();
    for i in 0..a.len() {
        let f = a.by_index_raw(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        if let Some(rel) = f.enclosed_name() {
            entries.push((i, rel, f.size()));
        }
    }
    let project = entries
        .iter()
        .filter(|e| is_project(&e.1))
        .min_by_key(|e| (e.1.components().count(), e.1.to_string_lossy().to_lowercase()))
        .map(|e| e.1.clone())
        .ok_or_else(|| "it holds no AQUILA or legacy FLP project".to_string())?;
    total.store(entries.iter().map(|e| e.2).sum(), Ordering::Relaxed);
    let stem = zip.file_stem().map_or_else(|| "Project".to_string(), |s| s.to_string_lossy().into_owned());
    let beside = zip.parent().map(|d| if d.as_os_str().is_empty() { Path::new(".") } else { d });
    let mut first_error = None;
    for base in beside.into_iter().chain(fallback) {
        let Some(dest) = folder_for(base, &stem, &entries, &project) else { continue };
        done.store(0, Ordering::Relaxed);
        match unzip_into(&mut a, &entries, &dest, done) {
            Ok(()) => return Ok(dest.join(&project)),
            Err(e) => {
                first_error.get_or_insert(e);
            }
        }
    }
    Err(first_error.unwrap_or_else(|| "no folder to unzip it into".into()))
}

/// The folder under `base` the zip goes into: `<stem>`, else `<stem> (2)` and on. Existing extraction folders are preserved; a fresh folder is reserved atomically.
fn folder_for(base: &Path, stem: &str, _entries: &[Entry], _project: &Path) -> Option<PathBuf> {
    std::fs::create_dir_all(base).ok()?;
    for n in 1..10000 {
        let d=if n==1{base.join(stem)}else{base.join(format!("{stem} ({n})"))};
        match std::fs::create_dir(&d) {Ok(())=>return Some(d),Err(e) if e.kind()==io::ErrorKind::AlreadyExists=>continue,Err(_)=>return None}
    }
    None
}

/// Writes the zip's files into `dest`; a file already there with the same size is kept. Each file is written beside
/// its place, then moved in, so a stopped unzip leaves no part file under a real name.
fn unzip_into<R: Read + io::Seek>(a: &mut zip::ZipArchive<R>, entries: &[Entry], dest: &Path, done: &AtomicU64) -> Result<(), String> {
    let err = |p: &Path, e: &dyn std::fmt::Display| format!("{}: {e}", p.display());
    std::fs::create_dir_all(dest).map_err(|e| err(dest, &e))?;
    for (i, rel, size) in entries {
        let out = dest.join(rel);
        if std::fs::metadata(&out).is_ok_and(|m| m.is_file() && m.len() == *size) {
            done.fetch_add(*size, Ordering::Relaxed);
            continue;
        }
        if let Some(d) = out.parent() {
            std::fs::create_dir_all(d).map_err(|e| err(d, &e))?;
        }
        let mut part = out.clone().into_os_string();
        part.push(".part");
        let part = PathBuf::from(part);
        let mut f = a.by_index(*i).map_err(|e| err(rel, &e))?;
        let written = File::create(&part).and_then(|w| copy(&mut f, io::BufWriter::new(w), done)).and_then(|()| std::fs::rename(&part, &out));
        if let Err(e) = written {
            let _ = std::fs::remove_file(&part);
            return Err(err(&out, &e));
        }
    }
    Ok(())
}

/// Copies `r` into `w`, counting the bytes into `done`.
fn copy(r: &mut impl Read, mut w: impl Write, done: &AtomicU64) -> io::Result<()> {
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        w.write_all(&buf[..n])?;
        done.fetch_add(n as u64, Ordering::Relaxed);
    }
    w.flush()
}

// --- Writing a zip ---

/// What goes into the zip besides the project file: each sample file once, under the name it has in the zip.
#[derive(Default)]
pub struct Pack {
    /// The project file's name in the zip (no sample takes it).
    pub project: String,
    pub files: Vec<(PathBuf, String)>,
    /// Samples the project names that were not found (file names).
    pub missing: Vec<String>,
}

impl Pack {
    /// Takes the samples of the project being written; a sample renamed in the zip (two files of one name) gets that
    /// name in the project file too, so it is found by name once unzipped.
    pub fn gather(&mut self, st: &crate::settings_file::SettingsStore, p: &mut os_model::Project) {
        for ch in &mut p.channels {
            let Some(stored) = ch.sample_path.clone().filter(|s| !s.trim().is_empty()) else { continue };
            let real = PathBuf::from(crate::plugins::resolve_sample(st, &stored));
            if !real.is_file() {
                self.missing.push(file_name(&stored).to_string());
                continue;
            }
            let name = match self.files.iter().find(|(f, _)| *f == real) {
                Some((_, n)) => n.clone(),
                None => {
                    let n = self.free_name(&real.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()));
                    self.files.push((real, n.clone()));
                    n
                }
            };
            let old = file_name(&stored);
            if old != name {
                let path=format!("{}{name}", &stored[..stored.len() - old.len()]);
                for w in &mut p.audio_warps{if w.rendered==stored{w.rendered=path.clone();}}
                ch.sample_path = Some(path);
            }
        }
        for vocal in &mut p.vocal_edits {
            let original=PathBuf::from(crate::plugins::resolve_sample(st, &vocal.source));
            let rendered=PathBuf::from(crate::plugins::resolve_sample(st, &vocal.rendered));
            let owner=p.channels.iter().find(|c|c.index==vocal.channel).and_then(|c|c.sample_path.as_deref()).and_then(|path|{let real=PathBuf::from(crate::plugins::resolve_sample(st, path));if real==original{Some(false)}else if real==rendered{Some(true)}else{self.files.iter().find(|(_,name)|name==&file_name(path)).and_then(|(real,_)|if real==&original{Some(false)}else if real==&rendered{Some(true)}else{None})}});
            let current=vocal.render_key==crate::vocal_key::key(vocal);
            for stored in [&mut vocal.source,&mut vocal.rendered]{
                if stored.is_empty(){continue;}let real=PathBuf::from(crate::plugins::resolve_sample(st, stored));
                if !real.is_file(){self.missing.push(file_name(stored).to_string());continue;}
                let name=if let Some((_,name))=self.files.iter().find(|(p,_)|p==&real){name.clone()}else{let name=self.free_name(&real.file_name().unwrap_or_default().to_string_lossy());self.files.push((real,name.clone()));name};
                *stored=name;
            }
            if current{vocal.render_key=crate::vocal_key::key(vocal);}
            if let Some(rendered)=owner{if let Some(c)=p.channels.iter_mut().find(|c|c.index==vocal.channel){c.sample_path=Some(if rendered{vocal.rendered.clone()}else{vocal.source.clone()});}}
        }
        for warp in &mut p.audio_warps {
            let real=PathBuf::from(crate::plugins::resolve_sample(st, &warp.source));
            if !real.is_file(){self.missing.push(file_name(&warp.source).to_string());continue;}
            let name=if let Some((_,name))=self.files.iter().find(|(p,_)|p==&real){name.clone()}else{
                let name=self.free_name(&real.file_name().unwrap_or_default().to_string_lossy());self.files.push((real,name.clone()));name
            };
            warp.source=name;
        }
    }

    /// `name`, else `<stem>_2.<ext>` and on, so no two files in the zip share a name (case does not count).
    fn free_name(&self, name: &str) -> String {
        let taken = |n: &str| n.eq_ignore_ascii_case(&self.project) || self.files.iter().any(|(_, f)| f.eq_ignore_ascii_case(n));
        if !taken(name) {
            return name.to_string();
        }
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s, format!(".{e}")),
            _ => (name, String::new()),
        };
        (2..).map(|n| format!("{stem}_{n}{ext}")).find(|n| !taken(n)).expect("a free name")
    }
}

/// Writes the zip: the project file, then the samples, side by side at its top. Written beside `zip`, then moved in.
pub fn write_zip(zip: &Path, flp: &[u8], pack: &Pack, done: &AtomicU64, total: &AtomicU64) -> Result<(), String> {
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    let sizes: Vec<u64> = pack.files.iter().map(|(f, _)| std::fs::metadata(f).map_or(0, |m| m.len())).collect();
    total.store(flp.len() as u64 + sizes.iter().sum::<u64>(), Ordering::Relaxed);
    let options = |name: &str, size: u64| {
        let stored = Path::new(name).extension().is_some_and(|e| STORED.iter().any(|s| e.eq_ignore_ascii_case(s)));
        SimpleFileOptions::default()
            .compression_method(if stored { CompressionMethod::Stored } else { CompressionMethod::Deflated })
            .large_file(size >= u64::from(u32::MAX))
    };
    let mut part = zip.as_os_str().to_owned();
    part.push(".part");
    let part = PathBuf::from(part);
    let write = || -> Result<(), String> {
        let mut w = zip::ZipWriter::new(io::BufWriter::new(File::create(&part).map_err(|e| e.to_string())?));
        w.start_file(pack.project.as_str(), options(&pack.project, flp.len() as u64)).map_err(|e| e.to_string())?;
        w.write_all(flp).map_err(|e| e.to_string())?;
        done.fetch_add(flp.len() as u64, Ordering::Relaxed);
        for ((file, name), size) in pack.files.iter().zip(&sizes) {
            let mut r = File::open(file).map_err(|e| format!("{}: {e}", file.display()))?;
            w.start_file(name.as_str(), options(name, *size)).map_err(|e| e.to_string())?;
            copy(&mut r, &mut w, done).map_err(|e| format!("{}: {e}", file.display()))?;
        }
        w.finish().map_err(|e| e.to_string())?.flush().map_err(|e| e.to_string())?;
        std::fs::rename(&part, zip).map_err(|e| e.to_string())
    };
    let r = write();
    if r.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    r
}

// --- Both ---

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("os_zipped_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut w = zip::ZipWriter::new(File::create(path).unwrap());
        for (n, b) in files {
            w.start_file(*n, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(b).unwrap();
        }
        w.finish().unwrap();
    }

    fn names(zip: &Path) -> Vec<String> {
        let mut a = zip::ZipArchive::new(File::open(zip).unwrap()).unwrap();
        (0..a.len()).map(|i| a.by_index(i).unwrap().name().unwrap().to_string()).collect()
    }

    /// A zip unzips beside itself and the project nearest its top is the one opened; names leaving the folder are
    /// skipped; unzipping it again uses the same folder, and a folder holding other files of those names is not used.
    #[test]
    fn vocal_portable_pack_preserves_original_and_render_with_colliding_names(){
        let dir=std::env::temp_dir().join(format!("lemon-vocal-pack-{}",std::process::id()));let a=dir.join("source");let b=dir.join("render");std::fs::create_dir_all(&a).unwrap();std::fs::create_dir_all(&b).unwrap();
        let original=a.join("Voice.wav");let rendered=b.join("Voice.wav");std::fs::write(&original,b"original source").unwrap();std::fs::write(&rendered,b"tuned cache").unwrap();
        let mut p=crate::projects::test_project();let channel=p.channels[0].index;p.channels[0].sample_path=Some(rendered.to_string_lossy().into_owned());
        let mut edit=os_model::vocal::Edit{channel,source:original.to_string_lossy().into_owned(),source_hash:"0".repeat(64),rendered:rendered.to_string_lossy().into_owned(),render_key:String::new(),rate:16000,frames:16000,f0:vec![0.;201].into(),notes:vec![],curve:vec![],timing:vec![(0.,0.),(1.,1.)],enabled:true,strength:1.,formant:1.};edit.render_key=crate::vocal_key::key(&edit);p.vocal_edits.push(edit);
        let mut pack=Pack{project:"Vocal.lmn".into(),..Default::default()};pack.gather(&crate::settings_file::SettingsStore::memory(), &mut p);
        assert_eq!(pack.files.len(),2);assert!(pack.missing.is_empty());let edit=&p.vocal_edits[0];assert_ne!(edit.source,edit.rendered);assert_eq!(edit.render_key,crate::vocal_key::key(edit));
        assert!(pack.files.iter().any(|(p,n)|p==&original&&n==&edit.source));assert!(pack.files.iter().any(|(p,n)|p==&rendered&&n==&edit.rendered));
        let loaded=os_flp::project::load(&os_flp::project::create(&p).unwrap()).unwrap();assert_eq!(loaded.vocal_edits,p.vocal_edits);
    }

    #[test]
    fn unzips_beside_the_zip() {
        let d = temp("unzip");
        let zip = d.join("Song pack.zip");
        make_zip(&zip, &[("old/Backup.flp", b"b"), ("Song.flp", b"song"), ("Kick.wav", b"kick"), ("../evil.txt", b"x")]);
        let (done, total) = (AtomicU64::new(0), AtomicU64::new(0));
        let project = unzip(&zip, None, &done, &total).unwrap();
        assert_eq!(project, d.join("Song pack").join("Song.flp"));
        assert_eq!(std::fs::read(d.join("Song pack/Kick.wav")).unwrap(), b"kick");
        assert!(!d.join("evil.txt").exists());
        assert_eq!(done.load(Ordering::Relaxed), total.load(Ordering::Relaxed));
        assert_ne!(unzip(&zip, None, &done, &total).unwrap(), project, "each import preserves existing extracted edits");
        std::fs::write(d.join("Song pack/Kick.wav"), b"another kick").unwrap();
        assert_eq!(unzip(&zip, None, &done, &total).unwrap(), d.join("Song pack (3)").join("Song.flp"));
        make_zip(&d.join("empty.zip"), &[("a.wav", b"a")]);
        assert!(unzip(&d.join("empty.zip"), None, &done, &total).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Two samples of one name: the second is renamed, in the zip and in its project file.
    #[test]
    fn names_in_the_zip_are_unique() {
        let mut p = Pack { project: "Song.flp".into(), ..Default::default() };
        p.files.push((PathBuf::from("a/Kick.wav"), "Kick.wav".into()));
        assert_eq!(p.free_name("kick.WAV"), "kick_2.WAV");
        assert_eq!(p.free_name("Song.flp"), "Song_2.flp");
        assert_eq!(p.free_name("Snare.wav"), "Snare.wav");
    }

}

#[cfg(test)]mod audit_tests {use super::*;
 #[test]fn audit_updated_zip_equal_lengths_gets_new_contents(){let d=std::env::temp_dir().join(format!("audit-zip-fixed-{}",std::process::id()));std::fs::create_dir_all(&d).unwrap();let path=d.join("Song.zip");let make=|bytes:&[u8]|{let mut w=zip::ZipWriter::new(File::create(&path).unwrap());w.start_file("Song.lmn",zip::write::SimpleFileOptions::default()).unwrap();w.write_all(bytes).unwrap();w.finish().unwrap();};let done=AtomicU64::new(0);let total=AtomicU64::new(0);make(b"old");let a=unzip(&path,None,&done,&total).unwrap();make(b"new");let b=unzip(&path,None,&done,&total).unwrap();assert_ne!(a,b);assert_eq!(std::fs::read(a).unwrap(),b"old");assert_eq!(std::fs::read(b).unwrap(),b"new");std::fs::remove_dir_all(d).unwrap();}
}
