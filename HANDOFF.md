# AQUILA — Handoff

Status as of 2026-11-12. This document covers what this project is, what was done, where everything lives, how to build and test it, and what is left.

---

## 1. What AQUILA is

AQUILA is a Rust digital audio workstation made by merging two lines of development:

| Source | Repository | Role in AQUILA |
| --- | --- | --- |
| **ML Studio** (Michael Lane / Dorfhaus) | https://github.com/Dorfhaus/ML-Studio | The base. Its interface, mixer, browser, transport, menus and file format win wherever the two differ. |
| **Lemon Studio** (ItsAlphaNeon and contributors) | https://github.com/ItsAlphaNeon/Lemon-Studio | ML Studio's ancestor. Its newer commits (through November 2026) were merged in, including multi-core mixing and new plugins. |

The merged result was renamed **AQUILA** and published to **https://github.com/ajdohaxhia/ADE-Studio**.

---

## 2. Repository and branches

**Local checkout:** this folder (the one containing this file).

**Git remotes:**

| Remote | URL | Use |
| --- | --- | --- |
| `origin` | https://github.com/Dorfhaus/ML-Studio.git | ML Studio (the fork with 48 own commits) |
| `upstream` | https://github.com/ItsAlphaNeon/Lemon-Studio.git | Lemon Studio |
| `ade` | https://github.com/ajdohaxhia/ADE-Studio.git | **AQUILA — where work is pushed** |

**Branches:**
- The local branch is `michael-lane`.
- Pushes go to both `ade/michael-lane` and `ade/main`. `main` was force-replaced once with the merge, at the owner's request; every push since has been a normal fast-forward.

**Key commits (newest first):**

| Commit | What |
| --- | --- |
| `ca8e151e` | Resolve the remaining merge test failures and add this handoff |
| `4b00b476` | Retire Lemon Studio UI tests that AQUILA's ML Studio interface replaces |
| `15e1542e` | Fix more merge regressions: undo, rack, menus, looping |
| `aa976690` | Fix merge regressions in the engine, sampler and mixer |
| `fb9f17aa` | **The merge:** Lemon Studio upstream into ML Studio, rename to AQUILA |
| `0e9cc091` | Last ML Studio commit before the merge (the fork's tip) |

- `fb9f17aa^1` is ML Studio and `fb9f17aa^2` is Lemon Studio, so `git show fb9f17aa^2:<path>` gives upstream's version of any file.
- All 48 Dorfhaus commits and Lemon Studio's history are preserved.

To publish:

```bash
git push ade HEAD:michael-lane HEAD:main
```

---

## 3. Layout of the codebase

| Path | Contents |
| --- | --- |
| `crates/os-app` | The application: windows, editors, browser, mixer, rack, transport, history (undo), menus. Binary target `lemonstudio`. |
| `crates/os-engine` | Audio engine: voices, mixing, multi-core rendering (`engine/multicore.rs`), plugin delay compensation |
| `crates/os-model` | Project model shared by everything (`MixerTrack`, `Channel`, targets in `target.rs`) |
| `crates/os-flp` | FLP / `.lmn` / `.lemon` reading and writing, legacy conversion |
| `crates/os-ui`, `os-theme` | Drawing, widgets, themes |
| `crates/os-host`, `os-clap`, `os-vst2`, `os-vst3` | Third-party plugin hosting |
| `plugins/*` | Built-in instruments and effects (sampler, ML Plasma, Tesla, EQ…) |
| `tools/bundle-app` | macOS `.app` bundler |
| `tools/logo` | Icon generation from `crates/os-app/assets/logo/aquila.png` |
| `docs/assets/aquila/` | README logo and screenshots |
| `README.md` | Rewritten for AQUILA (features, building, shortcuts, credits) |

**Naming notes:**
- `version::NAME` is `"AQUILA"`.
- Built-in plugins are mostly named "ML …"; a few keep their "Lemon …" names (Flanger, Phaser, Vocoder, Drop, the controllers).
- Mixer tracks are named "Bus N" everywhere.
- **Project files:** AQUILA saves `.lmn` (`project_files::EXT`) and opens `.lmn`, `.lemon` and `.flp`.

---

## 4. Building and testing (this machine)

### Why there is a special setup
1. **Paths with spaces break the build.** The original checkout lived under `~/Library/Application Support/...`; the active checkout is now `/Users/ade/Downloads/AQUILA/ML-Studio`. The bundled LAME (autotools/make) and ML Plasma native builds fail on paths with spaces.
2. **ML Plasma's build needs 20 GiB free** before it will compile.
3. **The selected Xcode is on an external drive whose path has spaces.**

### The setup in use
- An external-SSD **sparse disk image mounted at `/Volumes/aquilabuild`** (about 28 GB, no spaces in the path).
- **`/Volumes/aquilabuild/chk.sh`** is the helper used for every build and test. It:
  1. rsyncs `/Users/ade/Downloads/AQUILA/ML-Studio/` (corrected from the previous scratch checkout during continuation) to `/Volumes/aquilabuild/tree` (excluding `.git` and `target`), and
  2. runs cargo there with:
     ```
     DEVELOPER_DIR=/Library/Developer/CommandLineTools PYTHON=python3 CARGO_INCREMENTAL=0
     LEMON_STORAGE_CONFIG=/Volumes/aquilabuild/work/storage.json CARGO_TARGET_DIR=/Volumes/aquilabuild/target
     ```
- **`/Volumes/aquilabuild/work/storage.json`** = `{"minimum_free_gib": 6, "maximum_cache_gib": 0}` lowers ML Plasma's free-space minimum. This is for local runs only and must not go into release builds.
- **`CARGO_INCREMENTAL=0`** was added after the incremental cache (15 GB) filled the image. If space runs out, delete `/Volumes/aquilabuild/target/debug/incremental`.

Examples:

```bash
/Volumes/aquilabuild/chk.sh test -p os-app --bins -- mixer::
```

```bash
/Volumes/aquilabuild/chk.sh test --workspace --lib --bins --tests --no-fail-fast
```

**Gotcha:** the os-app test binary is `--bins` (named `lemonstudio`), not `--bin os-app`.

### Test-comparison helpers (in `/Volumes/aquilabuild/work/`)

| File | Purpose |
| --- | --- |
| `base-sorted.txt` | **Baseline:** the 110 tests that already failed in ML Studio before the merge |
| `newfail.sh <cargo test args>` | Runs tests, prints pass/fail totals and **only failures not in the baseline** |
| `ignore.py "reason" mod::tests::name ...` | Adds `#[ignore = "reason"]` to the named tests (run from the repo root) |
| `last.log` | Output of the most recent `newfail.sh` run |
| `up-mixer.rs`, `fork-mixer.rs` | Upstream / fork copies of `mixer.rs` used during the merge |

---

## 5. What was done

### 5.1 The merge (`fb9f17aa`)
- Lemon Studio's new commits were merged into ML Studio, and conflicts were resolved in favour of ML Studio's interface and behaviour.
- Duplicate definitions from both sides were reconciled (`Sprite`, `MHit::Input`, `Separator`, `DETACHABLE`, `snap_edge`…).
- Upstream's transport and browser were adapted to ML Studio's with shims.
- Rebranded to AQUILA:
  - names and packaging
  - the icon set (`.png`, `.ico`, `.icns`, nightly variants)
  - the splash `ml-splash.png`, generated from `crates/os-app/assets/logo/aquila.png`
- README rewritten with screenshots.

### 5.2 Regression fixes after the merge

**Engine and sampler** (`aa976690`)
- Restored ML Studio's Playlist audio-voice reconciliation when editing during playback. Upstream's block now skips audio-clip voices (`clip_origin`).
- Instruments' extra outputs route to their mixer tracks (`InstOut::aux`, `instruments_out`).
- Lemon Sampler: the `on` switch keeps its live value; per-note drive is implemented (`Voice::drive`, `note_drive`).
- Result: all 426 engine tests pass, and 80/80 sampler tests pass.

**Mixer** (`aa976690`)
- `slot_cpu` was 10 entries long while slots are 64, which crashed on slots past 9. Fixed in `os-model`, `os-flp` and the engine's `slot_smart`.
- Track moves remapped automation twice (ML Studio's `retarget` plus upstream's `follow_places`), which swapped targets back. Only the upstream pass and ML Studio's peak-link remap remain.
- Restored upstream track-menu commands inside ML Studio's menu: rename, random/gradient colour, icons, audio-track assign, routing, select, groups, move, insert one/multiple/from state, File ▸ states, audio editor.
- Help pages for "ML …" plugins (`help_links::plugin_page`).

**Undo, rack, menus** (`15e1542e`)
- **Plugin-window gestures:** ML Studio's per-value undo and Lemon Studio's per-gesture undo both ran. The per-gesture one is kept (`params.rs`).
- **Duplicate steps removed:** mixer colour and track moves recorded twice. Single moves are now named "Move mixer track".
- **Channel delete via the D key** now goes through `rack::delete_step`, so it is undoable.
- **`State::rows_moved` is consumed again** in `plugin_tick`. Undo/redo brings removed instruments back, including instrument-stack children.
- **Remote-control links follow moved channels** (`rack::reorder` calls `remote::follow`).
- **Advanced looping:** loop points play, save and burn (`row_loops`, `write_loops`).
- **"Color selected ▸ All"** works (`patterns::apply_swatch`).
- **Rack menus:** the channel menu gained Change icon, waveform, Wrap in container and Assign to new instrument track. The options menu gained "Selected channels ▸" and "Options ▸" submenus, keeping ML Studio's compact top level of 10 entries or fewer.
- **Playlist options:** Quantize song jumps and the performance-mode entries are placed again. The "Reset empty tracks" label is fixed.
- **Collaborate** got a real `MenuCmd`. The stray "MIDI chords" placeholder is removed.
- **Other:**
  - Saved-by text is "AQUILA x.y.z".
  - Vocoder plugin category.
  - `LINKED_EFFECTS` order matches the built-ins.
  - "Bus N" naming in the rack and mixer views.

**Tests adapted rather than code changed** (where ML Studio's design deliberately differs):
- the insert-multiple form
- the reset confirmation
- the typing keyboard being off by default
- the TRACK box doing free-drag routing
- the categorised plugin menus
- the in-app Change Log instead of "What's new"

### 5.3 Retired tests
About 50 Lemon Studio tests check UI that AQUILA replaces with ML Studio's. They are marked `#[ignore = "<reason>"]`, so they are easy to find and re-enable.

| Reason | Tests |
| --- | --- |
| Group fader semantics | 7 tests in `history.rs` |
| Animations | ripples, fades, captions, popup timing |
| Browser and drag tag | drag-tag tests, browser rows, channel-states browse, init browser |
| Windows | `win_clamp`, detach tests, detached editor layout, designer fonts |
| Plugin picker and palette | picker previews, palette, plugin manager |
| Rack view layout | 2 tests in `rack.rs` |
| Containers | 2 tests: ML built-ins saved as CLAP references need installed bundles; ML Studio's browser rows |
| ML Studio test replaced | `params::forum_editor_gesture_undo_redo`, covered by `control::tests::edits_in_a_plugin_s_window_are_undo_steps` |

To list them all:

```bash
grep -rn '#\[ignore = ' crates plugins tools
```

---

## 6. Test status before continuation

**Full workspace** (`--workspace --lib --bins --tests`):
- 2941 passed, 129 failed, 119 ignored.
- Of the 129 failures, **104 were already failing in ML Studio** (the baseline) and **25 are new** since the merge, down from 134.

### 6.1 The 25 remaining new failures

**Recording (likely a real bug, investigate first)**
- `smart_recording::tests::smart_mode_arms_without_a_question_and_writes_automation_clips`
- `record::tests::records_mixer_controls_the_tempo_and_plugin_parameters`
- `record::tests::takes_the_clip_just_before_in_song_mode`
- `record::tests::option_buttons_menus_and_keys` (expects Count-in length / Pre-roll entries)
- `track_rec::tests::an_armed_track_records_into_the_playlist`

**What's known about the recording failures:**
- `record.rs`, `smart_recording.rs`, `mixer_recording.rs`, `track_rec.rs` and `prefs.rs` are **identical to ML Studio's**, and so is the `Cmd::Record` handler in `playlist.rs`.
- Test preferences are thread-local (`settings_file.rs`), so this is not leakage between tests.
- `Cmd::Record(Opt::Automation)` toggles automation **off** (it defaults to on), yet ML Studio's test expects it on after `toggle_record`. In ML Studio's baseline this test passed, so something reached through `App::new` or `toggle_record` in merged code (app.rs or the transport) must behave differently.
- `records_mixer_…` also shows a recording making two undo steps: "Recorded items" plus "Create automation clip" from smart recording.

**App (os-app)**
- `app::tests::tempo_and_time_settings_are_undo_steps`: dragging the tempo display doesn't change the tempo in ML Studio's transport layout (`tempo_digits` hit area). This could be a UI difference or a bug.
- `app::tests::alt_click_resets_controls_without_dragging`: Alt+click gives 0.8, expected 0.23 (reset value).
- `app::tests::selection_fixes_mixer_reset_and_group_solo`: one value 0.8 vs 0.95.
- `app::tests::piano_ctrl_drag_selects_in_draw`: Ctrl+drag in Draw mode.
- `app::tests::main_menu_titles_switch_on_first_press`: expects an enabled "Undo history" entry.
- `app::tests::settings_tabs_can_be_hidden_and_restored_from_visibility`: an extra tab index (2) in the tab order.
- `app::tests::channel_rack_window`, `app::tests::rack_preview_mute_button_and_preview_under_the_steps`: rack window layout; probably UI differences.
- `init::tests::create_automation_clip_lists_the_control_while_the_check_is_on`: the "Automation" rack group or filter.
- `playlist::tests::renaming_a_channel_clip_renames_its_channel`: the rename box targets `AudioClip(0)` instead of `Rack(Channel(1))`.
- `playlist::tests::slot_window_menu_entries`: a plugin-window menu entry (`Plugin(Picker(Fx(Some(2))))`).
- `remote::tests::view_remote_control_lists_the_links`: the VIEW menu's Remote control entry.

**Other crates**
- `editor::tests::drum_loop_stretch_and_noise_gating`, `editor::tests::time_stretch_and_pitch_shift` (`plugins/tesla`)
- `editor::tests::tab_wheel_moves_a_step_per_notch_and_adds_up_fractions`, `editor::tests::the_knob_captions_stay_clear_of_band_1s_knobs` (`plugins/lemon-parametric-eq-2`)
- `editor::tests::the_newer_pages_wordmarks_end_6_px_from_the_right_edge` (`plugins/lemon-small-fx`; the wordmark is still "Lemon Filter")
- `project::legacy::tests::an_old_file_opens_and_saves_converted` (`crates/os-flp`): legacy file conversion; worth checking, because it affects opening old projects.
- `tests::a_nightly_is_an_app_of_its_own` (`tools/bundle-app`): Info.plist / `CFBundleIconName` after the rename.
- `tests::paints_text_and_shapes` (`crates/os-ui`): text width assertion (`w > 40.0`).

### 6.2 Suggested order
1. Recording (section 6.1): the most likely user-facing bug.
2. `project::legacy::...` (opening old projects).
3. `tempo_and_time_settings_are_undo_steps`, `alt_click_resets...` and `renaming_a_channel_clip...`, which may be real.
4. Bundle-app nightly test (packaging).
5. The rest: decide case by case whether to fix the code, adapt the test to ML Studio's design, or retire it with `ignore.py` and a reason.

---

## 7. Merge patterns to watch for

These caused most of the regressions, so check for them when touching merged code:

- **Double handling:** ML Studio and Lemon Studio each added their own mechanism for the same thing (undo recording, target remapping, gesture handling). Look for a fork-side `history::record` or remap inside code that upstream already wraps with `control::edit_app` or `follow_places`.
- **Lost glue:** functions that exist but are never called after the merge, such as `rack::moved_link` and the consumer of `State::rows_moved`. A quick check is `grep` for definitions with no callers.
- **Menus:** the merge usually kept ML Studio's menu builders and dropped upstream entries. Compare with `git show fb9f17aa^2:<file>`.
- **Array sizes:** ML Studio has 64 mixer slots; some upstream code assumed 10.
- **Renames:** tests and lookups still using "Lemon X" names, `.lemon` extensions, or "Insert N" instead of "Bus N".

---

## 8. Open decisions for the owner

- Whether to keep or re-enable any retired Lemon Studio UI tests, which would mean porting those UI features into ML Studio's interface.
- Whether the "Bus N" naming (now everywhere) is wanted, or ML Studio's earlier "Insert N" in the rack and mixer views.


## 9. Continuation on 2026-11-12

This section supersedes the remaining-work list in section 6. These changes were committed and pushed as `ca8e151e`.

### Fixes and test updates

- Corrected `/Volumes/aquilabuild/chk.sh` to sync the active Downloads checkout instead of the previous scratch checkout.
- Recording: audio clips honor their explicit/inherited mixer route; smart capture no longer also writes pattern controls. Remote faders, plugin parameters and container levels use smart capture, including Start on input. Clip extension preserves ML Studio's gap behavior but avoids occupied rows, including assigned instrument lanes.
- Grouped mixer fader resets preserve every changed fader in one undo step; resetting default values adds no step.
- Restored EDIT → Undo history and Current project → Remote control. Project data remains immediately before Samples.
- Legacy imports that are unchanged after conversion preserve the event stream rather than append default-only native chunks.
- Nightly packaging uses AQUILA Nightly as the display name and has one microphone permission key; its test checks the actual ICNS icon setup.
- Restored Tesla's right-click Drum Stretch action and fractional wheel stepping over the modern EQ type selector. Fixed effect wordmark right alignment.
- Adapted tests for intentional ML Studio behavior: tempo drag speed/scaling, individual audio-clip names, instrument palette, rack preview/LED arrangement, additive piano selection, Recording settings tab, categorized effect menus, template rack groups, recording filters/gap semantics, and modern plugin geometry. No new ignored tests were added.
- Made Chop host-audio tests initialize at their explicit 48 kHz render rate instead of inheriting the machine's 44.1 kHz device rate.
- New integration tests cover smart plugin automation without duplicate pattern controls and remote fader/container recording, including Start on input and disabled filters.

### Verification

The first full workspace run reported 2964 passed, 107 failed, 119 ignored. All original 25 listed merge failures were resolved. The three additional nonbaseline failures were a browser section ordering assertion (corrected) and two Chop host tests being investigated for a device-rate/fixture-rate mismatch. Final verification is recorded below.


Final full workspace command:

```bash
/Volumes/aquilabuild/work/newfail.sh --workspace --lib --bins --tests --no-fail-fast
```

**2968 passed, 104 failed, 119 ignored. Zero failures outside the pre-merge baseline.** All 25 original remaining merge failures are resolved. The final run includes the new remote/container/plugin recording tests, assigned-lane collision coverage, and explicit-rate Chop tests. No new ignores were added. The 104 pre-existing failures remain; coverage percentage was not measured. `git diff --check` passes. Code review found no remaining actionable issues after the recording follow-ups.

Final log: `/Volumes/aquilabuild/work/last.log`. First continuation full run: `/Volumes/aquilabuild/work/continuation-first.log`. Chop rate diagnostic: `/Volumes/aquilabuild/work/chop-red.log`.

Next work, if requested: investigate the 104 baseline failures; do not classify them as unresolved merge regressions. The continuation's changes and this handoff are committed and pushed (`ca8e151e`).

---

## 10. Session paused (2026-11-12, evening): start here next time

### State
- Pushed: `ca8e151e` on `ade/main` and `ade/michael-lane`. Every merge regression is resolved; the full workspace had 2968 passed and 104 failed, and all 104 failures were already failing in ML Studio before the merge.
- **Uncommitted local work** (below) started on those 104 pre-existing failures. **None of it has been committed or pushed.**
- The build helper `/Volumes/aquilabuild/chk.sh` and `/Volumes/aquilabuild/work/newfail.sh` point at `/Users/ade/Downloads/AQUILA/ML-Studio` (fixed this session). The disk image `/Volumes/aquilabuild` must be mounted.

### Uncommitted changes, by file
| File | Change | Verified? |
| --- | --- | --- |
| `HANDOFF.md` | Out-of-date lines fixed (commit table, "not committed" notes); this section | n/a |
| `crates/os-app/src/vocal/mod.rs` | Test fixture: each test gets its own temp folder (parallel tests were rewriting one shared `Vocal.wav`, giving "Original recording changed during analysis") | Yes: all 14 `vocal::` tests pass |
| `crates/os-app/src/mpe.rs` | `mpe::chop` (Quick chop, Ctrl+U) now records an undo step (`Some("Piano roll edit")`); before, it couldn't be undone | Yes: `piano_keys::legato_chop_and_paste` passes |
| `crates/os-app/assets/native-generators-groups.json`, `native-effects-groups.json` | Factory plugin folders use the current "ML …" names (old names kept), so built-ins no longer land in "Unsorted" | Yes: all `plugin_menu::` tests pass |
| `crates/os-app/src/strip_silence.rs`, `comping.rs` | Tests search submenus (ML Studio's clip menu nests Strip silence and "Use selected time range in comp") | Yes: these 2 pass |
| `crates/os-app/src/rack.rs` (`play_project`) | While a take is soloed, its comp (parent) track is disabled in the playback copy only | **No: the test run was stopped before it finished.** Run `comping::` and `rack::` first. |

### First thing next session
```bash
cd /Users/ade/Downloads/AQUILA/ML-Studio
```

```bash
/Volumes/aquilabuild/chk.sh test -p os-app --bins -- comping:: rack::
```

```bash
/Volumes/aquilabuild/work/newfail.sh --workspace --lib --bins --tests --no-fail-fast
```

Then check that the pass count went up and that no failures outside the baseline appear, and commit and push:

```bash
git push ade HEAD:michael-lane HEAD:main
```

### The 104 pre-existing failures, triaged
Full list with messages: `/Volumes/aquilabuild/work/base-msgs.txt`. About 13 are fixed by the work above (vocal ×8, legato/chop, plugin_menu ×2, strip_silence, comping ×1, plus comping solo once verified).

**Worth checking (may be real bugs):**
- `app::tests::sampler_settings_save_and_reopen`: on save, `app.rs` (~line 2346) deliberately keeps the rack's "Follow global pitch" over the sampler's own "Affected by the main pitch". A change made only in the sampler UI may be lost. ML Studio did this on purpose; the right fix is to push host toggles into the plugin, then drop the override. Needs a decision.
- `osc3::tests::replace_puts_another_generator_on_a_channel`, `app::tests::instrument_template_routes_and_live_reroute`, `app::tests::pattern_click_and_channel_insert_anchor` (unwrap on None)
- `app::tests::add_audio_and_automation_clip_channels` (rack filter becomes Unsorted, expected All)
- `app::tests::piano_tools` (the Delete tool swipe removes nothing)
- `wav::tests::writes_edisons_chunk_set_in_order` (`plugins/tesla`): LIST INFO size 22 vs 18, probably the longer "AQUILA" software name
- `plugins/lemon-small-fx` `factory_presets_list_every_parameter_and_hold_fls_states`: a preset file is missing

**Design conflicts (ask the owner):** tool keys ("p" is Draw in the piano roll; the playlist test expects Paint), typing keyboard off by default (`step_edit` ×5), "Undo history" now in the EDIT menu (3 `history` tests expect the old menu), "Soft takeover" vs "Pickup" wording.

**Environment or stale (likely retire or adapt):** `ai_control` (local network), `tools/notices` (needs network), `tools/watermark` (header check), `splash`, theme, toolbar and window-layout pixel tests, `win_clamp` rounding, browser tests that assume Windows paths (`E:\Pro Samples`).

### Update (same evening, after resuming)
- The take-solo change in `rack.rs` is **verified**: 55/55 `comping::` and `rack::` tests pass.
- Also fixed:
  - `osc3` replace test (searches the categorised plugin menu)
  - `instrument_template_routes_and_live_reroute` (falls back to `HOME` and skips when the local file is missing)
  - retired `pattern_click_and_channel_insert_anchor` (Lemon Studio's two-column channel menu)
- **Full workspace:** 2983 passed, 88 failed, 120 ignored. Down from 104 pre-merge failures; none are new.
- **Flaky (not a regression):** `plugins/lemon-sampler` `params::tests::editor_edits_reach_the_sound_without_the_host` failed once in the full parallel run and passes alone (80/80, three runs). Sampler live values sit in a table shared by all tests, so parallel tests can collide. Fix by isolating the table per test or serialising these tests.
- **The two suspected bugs were checked (later the same evening):**
  - Rack filter after undo: undo restored the filter correctly. The app starts on Unsorted where the test assumed All, so the test now sets All. Real fix kept: `history::put_rack` marks the restored selection as seen, so Auto switch display filter can't move the restored filter again.
  - Piano roll Delete tool: works. In AQUILA, D duplicates notes by default (ML Studio's "D duplicates notes" option), and Ctrl+A keeps a manual time range. The test was adapted.
- **Full workspace now:** 2986 passed, 85 failed (all pre-merge), 120 ignored. Remaining list: `/Volumes/aquilabuild/work/last.log`; triage notes earlier in this section.
- **Reminder:** the build image lives at `/Volumes/SanDisk EXTREME SSD/aquila-build/aquilabuild.sparseimage`. Mount it with `hdiutil attach <that path>` (mounts at `/Volumes/aquilabuild`) and detach it before unplugging the SSD.

---

## 11. ML Studio v2026.11.12.1 and v2026.11.13.1 merged (2026-11-13)

**Owner's decision:** AQUILA now uses **ML Studio's audio engine** for instrument and mixer scheduling, not Lemon Studio's `multicore.rs`.

### What changed
- **Merged `origin/michael-lane` (`b7b0748a`)**: about 205 files, including ML Studio's vendored `cpal`, attached mixer / instrument tracks, the audio editor (`audio_edits`), the voice assistant, export advanced mode, latency presets, device extra outputs, worker limit and priority, smart disable, and its new parameter host events (`ParamEvent::Sync`).
- **Engine:** ML Studio's `instrument_workers`, `mix_workers` and `mixer_graph`, with its `render_instruments`, `mix_serial` and `mix_graph`. **`multicore.rs` was deleted**, along with `mix_track`, `instrument_job`, `route_out`, `LoadGate` and `Engine::set_threads`.
- **Kept from Lemon Studio** (ML Studio never touched these files):
  - the vocoder, filter, flanger and phaser effects; peak and formula controllers; formula; chain
  - the `auto_bypass` module and `rec.rs`
  - `run_slot`'s slot auto-bypass: ML Studio's callers now store its result in `fx.skipped`, and both callers pass the tick and the switch
  - pattern control points in `param_auto` (on ML Studio's `HashMap`)
  - the `inst_later` scheduled notes and the MPE MIDI channel on live notes (added back into ML Studio's `render_instruments`)
- **No longer active, by design** (their tests are retired with `#[ignore]` reasons):
  - Lemon Studio's per-track input monitoring, direct outputs and disk recording in the engine (AQUILA already used ML Studio's transport for these)
  - channel-level auto-bypass for hosted instruments
  - unfaded hosted instruments during pause
  - per-frame automated level ramps for hosted instruments
- **Devices:** ML Studio's `Source` trait, `Mix` and extra device channels. Lemon Studio's variable-width multi-output (`set_width`, `play(out, width)`) is gone. `transport::raise_priority` now forwards to ML Studio's `set_feeder_priority(2)`. Linux real-time priority for the callback is kept.
- **`params.rs` deliberately kept as AQUILA's (Lemon Studio's parameter lists and gesture undo).** It now handles `ParamEvent::Sync` as a host baseline update. **Not yet ported:** ML Studio's non-blocking batched parameter polling (`poll_index`, `param_values_job`, `POLL_BATCH`), which avoids audio stalls with large VST3 banks, plus its two tests. That's a worthwhile follow-up.
- **Prefs:** ML Studio's `SMART_DISABLE` (key `smart_disable_v1`, plugins sleep when idle) and AQUILA's renamed `AUTO_BYPASS` (key `smart_disable`, "Auto-bypass when silent") are separate. Both appear on Audio ▸ CPU, next to ML Studio's threaded generator/mixer switches, worker limit and priority.
- **Model:** `PlaylistTrack` has both ML Studio's `attached_mixer` and Lemon Studio's `TrackLink` (similar purpose; consider unifying later).
- **Packaging:** `github-release.json` and the NSIS version are 2026.11.13.1. The release JSON's download URLs still point at ML Studio's server (michaellanemusic.de), as they did before; they need AQUILA's own release URLs before publishing.

### Not verified
- **Windows-only code** (`device/asio.rs`, `plugin_window/win32.rs`) was resolved to ML Studio's side but **cannot be compiled on this Mac**. Build on Windows before a Windows release.

### Test status after the merge
**3058 passed, 103 failed, 133 ignored.** 21 failures are outside the old pre-merge baseline:
- **9 need ML Studio's signed native vocal engine on the machine** (the tests deliberately fail without it): `audio_edits::tests::*` (7), `warp::tests::legacy_tuned_warp_recovers_original_controls`, `zipped::tests::audio_edits_portable_pending_revision_keeps_history_and_source`. Not a merge regression; install the vocal engine to check them.
- **12 to look at next:**
  - `app::tests::audio_router_numbers_drag_both_directions` (no "Browser preview track" number on the page)
  - `app::tests::global_snap` (toolbar layout)
  - `app::tests::hotfix_scrolled_settings_keep_about_clickable`
  - `app::tests::playback_repaint_matches_full`, `app::tests::tempo_tapper_flash_repaints_the_pad` (repaint regions)
  - `app::tests::playlist_colorful_waveforms`
  - `attached_mixer::tests::attached_existing_instrument_records_midi_in_its_projected_lane`: ML Studio's attached-mixer recording against AQUILA's recording path. **Likely the most important.**
  - `audio_cfg::tests::asio_buffer_is_always_driver_owned`
  - `browser_preview::tests::browser_preview_options_and_playlist_menu`
  - `channel_bounce::tests::rack_features_compact_controls_and_filtered_piano_layout`
  - `help::tests::guide_menu_search_read_back_and_visibility` (a guide topic lacks category, keywords or paragraphs)
  - `mixer::tests::mixer_batch_insert_validates_and_restores_routing_and_selection` (AQUILA's track menu always shows "Insert multiple...")

### Update after fixing the merge's integration failures (2026-11-13)
- **Fixed:**
  - Recording into an attached instrument lane: the lane's own instrument clips no longer push the take to another row (`record.rs`).
  - Full-frame repaints on every playback tick: `win_clamp::keep_all` compares rounded positions.
  - The colour change was one step too many: plain `MCmd::Color` records through `set_track_color` only.
  - Mixer-track rename step name; sidechain source names ("Bus N").
  - "Insert multiple..." follows ML Studio's visibility switch.
  - About page name; a help guide topic with one paragraph; "AI assistant settings" in the label-dispatched menu list; VST3 test tolerates `Sync`.
  - Slot auto-bypass is recorded again (`fx.skipped` from `run_slot` in `mix_serial` and `mix_workers`).
- **Full workspace: 3053 passed, 100 failed, 133 ignored. 15 failures are outside the old baseline, and none are caused by the merge:**
  - **9 need ML Studio's signed native vocal engine installed** (`audio_edits::*`, `warp::legacy_tuned_warp_recovers_original_controls`, `zipped::audio_edits_portable_pending_revision_keeps_history_and_source`).
  - **6 fail in ML Studio's own v2026.11.13.1 as well** (checked by building a clean copy of `origin/michael-lane`): `app::tests::audio_router_numbers_drag_both_directions`, `app::tests::global_snap`, `app::tests::playlist_colorful_waveforms`, `audio_cfg::tests::asio_buffer_is_always_driver_owned`, `browser_preview::tests::browser_preview_options_and_playlist_menu`, `channel_bounce::tests::rack_features_compact_controls_and_filtered_piano_layout`.
- **How to check a test against pure ML Studio:**
  1. `git worktree add --detach /Volumes/aquilabuild/mltree origin/michael-lane`
  2. `rsync -a --delete --exclude .git --exclude target /Volumes/aquilabuild/mltree/ /Volumes/aquilabuild/tree/`. ML Plasma's CMake cache needs the `/Volumes/aquilabuild/tree` path.
  3. Run cargo there with the same environment as `chk.sh`.
  4. Remove the worktree afterwards. The next `chk.sh` run syncs AQUILA back.

## 12. Lemon Studio's 41 new commits merged (2026-11-13)

ML Studio had nothing new. Lemon Studio `main` had moved from `415eaf26` to `011a03f2` (41 commits: playlist bars
following the time signature, stacked edges on overlapping notes and clips, FL-style gestures, Make unique for audio
clips, Consolidate / Export tracks rendering hosted plugins, per-pattern rack loops, undo fixes, the equivalence
harness in `crates/os-app/src/equiv.rs`). 58 conflicts in 10 files.

### Choices made

- **Rack loops per pattern:** Lemon Studio's mechanism (`rack.loops_of`, `follow_pattern`, `load_loops`,
  `store_loops`) replaces ML Studio's `read_loops` / `write_loops`. `play_project` keeps ML Studio's expansion of every
  pattern's loops, with `loops_of` as the pattern the rack holds. `pattern_seen` is still used by `pattern_tick` for
  auto-collapse and channel selection only.
- **Playlist ruler and grid lines:** drawn from `pl::Bars` (Lemon Studio), keeping ML Studio's snap-sized grid lines
  and vintage ruler digits.
- **Playlist Select tool:** ML Studio's stays. Lemon Studio's Shift-toggle box and right-click-clears-selection were
  not taken, so `PlDrag::Marquee` has no `toggle` field. The test `playlist_tool_wheel_and_select_box` became
  `playlist_tool_wheel`, written against ML Studio's toolbar order.
- **Playlist solo:** `solo_in_effect` now compares against `solo_keep` (ML Studio's folder / group aware set), so a
  solo that no longer holds ends without putting old states back.
- **Clip painting:** ML Studio's translucent bodies and stroked outlines stay; Lemon Studio's `see` closure for
  automation clips is unused.
- **Exports:** `App::start_export_of(job, p, song)` (Lemon Studio's name) forwards to ML Studio's
  `start_project_export(p, job, song)`.
- **Equivalence harness:** `Transport::sent_for_tests` returns `None`. ML Studio's transport hands updates to the
  audio thread through a slot it empties and keeps no copy, so the harness's "engine holds the project" check is off.
- **Undo of a pattern edit** no longer switches the pattern shown (Lemon Studio's change).
- `Snap::Knob` holds `Option<f64>`; `history::knob_value` returns `Option<f64>`.

### Test status

`newfail.sh --workspace --lib --bins --tests --no-fail-fast` before the last three fixes: 3076 passed, 103 failed,
140 ignored, with 3 new failures, all in os-app. After fixing them, `newfail.sh -p os-app --bins`: 1466 passed, 90
failed, 107 ignored, and the failures outside the baseline are the same 15 as in section 11. The full workspace was
not rerun after those three fixes.

Use `--lib --bins --tests`: a bare `--workspace` also builds `plugins/lemon-wave/examples/verify_host.rs`, which does
not compile.

### Still open

- Windows-only code is not compiled.
- `packaging/github-release.json` still links ML Studio's binaries; there are no AQUILA builds to link.
- ML Studio's batched parameter polling (`poll_index`, `POLL_BATCH`) is not ported: AQUILA does not poll values
  every tick, it reads them when a list is built and when a re-read is due.

### Update (same day): full run and lock-free value reads

- Full workspace after the fixes above: 3079 passed, 100 failed, 140 ignored. The failures outside the baseline are
  the same 15 as in section 11.
- `params.rs` now uses `Plugin::param_values_job()` (from ML Studio's host) for those two reads when a plugin offers
  it: the snapshot is collected in `snapshots` and run after the instance is unlocked, so a large bank does not hold
  the audio thread. Plugins without it are read under the lock as before. The os-app tests are unchanged by this
  (1466 passed, 90 failed); no test exercises the snapshot path with a real VST3.
- The build image had been unmounted; remount with
  `hdiutil attach "/Volumes/SanDisk EXTREME SSD/aquila-build/aquilabuild.sparseimage"`.

## 13. ML Studio v2026.11.13.2 merged (2026-11-13)

ML Studio published `6be4ba4c` ("Publish complete DAW v2026.11.13.2"): the product is renamed from "Lemon Studio" to
"ML Studio" throughout (stock plugins too: "ML Limiter", "ML Sampler", ...), the sampler gains arpeggiator and echo
note effects in the engine (`engine/sampler_notes.rs`), preset loads build a new plugin instance, the user guide is
rewritten with its own detachable window, and saved custom window layouts arrive (`workspace_layout.rs`). 110
conflicts in 40 files; 52 were only the rename and were resolved by script (ours where ML Studio only renamed, theirs
with "ML Studio" turned into "AQUILA" where only AQUILA had renamed).

### Choices made

- **Names:** the app stays AQUILA (`version::NAME`, `os_paths::APP_NAME`, window titles, updater messages, the
  templates' factory category). The stock plugins take ML Studio's new "ML ..." names; the effects only AQUILA has
  keep theirs (Lemon Filter, Flanger, Phaser, Vocoder, Peak Controller, Formula Controller, Drop). The user guide text
  still says "ML Studio".
- **Engine:** ML Studio's sampler note effects are in, with AQUILA's extra fields kept on `Trigger`, `notes::Event`
  and `NoteLevels` (`clip_origin`, `at`, `stop`, `colour`, `swing`, `end`, `channel`). A channel that is off still
  starts no notes (the check is at the top of `start_plain_trigger`). `run_slot` smooths, then reports the skipped
  slot.
- **Lemon Sampler DSP:** Lemon Studio's restructured voice code stays; ML Studio's `echo_pitch` and `note_off_id`
  are added to it.
- **Playhead follow (`App::auto_scroll`):** rewritten once from both: ML Studio's `visual_tick` and its "not while
  dragging in the playlist" rule, Lemon Studio's piano roll following a pattern clip in song mode.
- **VIEW > Arrange windows:** Lemon Studio's layouts first, then ML Studio's saved custom layouts; the entry is
  always shown.
- **Preset loads:** ML Studio's prepared instance. An instance whose library is not among the listed plugins takes the
  state in place instead of being refused (`plugins::NO_LIBRARY`); one whose library changed is still refused.
- **Guide:** topics added for the seven AQUILA-only effects, which the guide's own test requires.
- **Help > About** stays AQUILA's (`MenuCmd`), not ML Studio's Settings page.
- `rack_additions_fit_channels...` allows the rack's top to sit a fraction of a pixel above the workspace's, because
  AQUILA keeps windows on whole pixels.

### Test status

`newfail.sh --workspace --lib --bins --tests --no-fail-fast`: 3120 passed, 100 failed, 140 ignored. Outside the
baseline: the 15 of section 11 plus `app::tests::transport_clock_digits_share_a_bottom_line`, which also fails in a
clean build of ML Studio v2026.11.13.2 on this machine.

### Still open

- Windows-only code is not compiled.
- `packaging/github-release.json` lists ML Studio's v2026.11.13.2 binaries; there are no AQUILA builds.
- The sampler's arpeggiator and echo were not listened to in the running app.

## 14. ML Studio v2026.11.13.3 merged (2026-11-13)

`47e3164e` ("Publish complete DAW v2026.11.13.3"): customisable wet/dry knobs in the Traditional Mixer, a plugin
download catalogue in the browser, a route picker for plugin windows, theme regions. 40 files, 4 real conflicts.

- `app.rs`: AQUILA's `channel_control` and ML Studio's `open_plugin_route_picker` both kept.
- The test `vintage_switch_restores_theme_and_keeps_workspace_controls`, which AQUILA had retired, stays retired.
- The installer script keeps AQUILA's name, at version 2026.11.13.3.
- `plugin_window`'s track box test expects "Bus 11": AQUILA names mixer tracks "Bus N".

### Test status

Full workspace: 3136 passed, 96 failed, 141 ignored, before the "Bus 11" fix (that test passes now). Outside the
baseline: the 16 of section 13 plus `plugin_downloads::tests::catalog_browser_search_and_layout`. That test searches
the catalogue for a plugin (oXygen) that has no macOS arm64 download, so it cannot pass on this machine; it was not
run against a clean ML Studio build.

### Update (same day): the guide says AQUILA

`docs/user-guide.md` (shown in the app's Help window) now says "AQUILA" where it said "ML Studio", topic titles and
their "Related:" links included, and its install topic points to https://github.com/ajdohaxhia/ADE-Studio instead of
ML Studio's download page. A later merge of ML Studio's guide will conflict wherever those lines change: take theirs
and repeat the replacement. os-app tests after the change: 1515 passed, 85 failed, the failures outside the baseline
unchanged.

## 15. ML Studio v2026.11.14.2 merged; AQUILA no longer contacts michaellanemusic.de (2026-11-14, in progress)

**State at handoff (nothing committed or pushed yet):**

- A merge of `origin/michael-lane` 5fedeac2 (v2026.11.14.2) is open (`.git/MERGE_HEAD`). All conflicts are resolved.
- The staged files plus these unstaged merge fixes compile cleanly (`chk.sh check --workspace --lib --bins --tests`):
  - `record.rs`: the AQUILA `Take.controls` field restored (struct + `open_take`), `.as_deref()` in a test.
  - `recording_merge.rs`: `schedule_controls` returns `(tick, kind, value)`; `kind` is the same word as
    `target::channel(ch, param)`. `Automation` literals gain `lfo: Default::default()`.
  - `smart_recording.rs`: `.as_deref()` on `history::undo` in tests.
  - `params.rs`: the test's `CapturePlugin` is `{desc, events, value}`.
  - `studio_helper.rs`: test no longer looks for the hidden Help menu row.
  - `plugins/smith/src/editor.rs`: `Cmd::BrowseSample | Cmd::SampleFit => 0` (Windows-only match; CI run
    43618382354 failed there; Linux and macOS were green).
- Test run of the merge state (`work/newfail.sh`, before the second change): passed 3178, failed 112. Besides the 17
  of section 14, **11 new failures**, to fix or explain before committing the merge:
  - Smart recording after the merge (likely the params/engine/record resolutions of this merge):
    `params::smart_plugin_recording_does_not_also_write_pattern_controls` (params.rs:1266),
    `remote::remote_fader_and_container_levels_record_in_smart_mode` (remote.rs:919),
    `record_lead::lead_fix_modes_timing_note_automation_gates_and_stop` (record_lead.rs:51, `smart_pending()` false),
    `engine::touch_override_resumes_a_flat_curve_and_survives_seek` (engine.rs:9623: the touched channel does not
    resume its automation value; see the `CtlTo::Channel` lookup in `automation_writing`).
  - `mixer::workflow_effect_bypass_preserves_selection_locks_and_undo` (mixer.rs:4013; `toggle_slot` resolution).
  - Expected from AQUILA's choices, adjust the tests: `views::menus::working_entries_have_their_command` (Melli is
    hidden), `help::tests::guide_catalog_links_images_and_stock_plugins` (guide says AQUILA, test expects "ML Studio").
  - Theme: `design::tests::classic_is_unchanged`, `fl::tests::the_light_mode_key_is_ignored` (os-theme; compare with
    ML Studio's 14.2 theme files).
  - `helper_install::tests::melli_archive_validation_and_owned_extraction` (helper_install.rs:171),
    `playlist_tracks::drops::tests::workflow_header_drop_modes_undo_reopen_and_routing` (log: "ML Plasma payload has
    an unexpected instrument identity").
  Run a failing test alone with `/Volumes/aquilabuild/chk.sh test -p os-app --bins <name> -- --nocapture`. A test
  that also fails in a clean ML Studio 14.2 checkout joins the baseline (`work/base-sorted.txt`).

**Second change, written but not yet compiled, to commit separately after the merge:**

- `updates.rs`: no update feed; `request()` does no network work; `RELEASES` = the AQUILA GitHub releases page;
  `PRODUCT` ids are `aquila-*`. `check()` kept (signed-update tests) but unused.
- `app.rs` (`Action::Updates`): opens `updates::RELEASES` in the browser.
- `feedback.rs`: Report a bug opens a new GitHub issue on ajdohaxhia/AQUILA, diagnostics in `?body=`.
- `vocal/engine.rs`: `BASE` is `None`; `install_at` returns "not available in AQUILA".
- `helper_install.rs`: `install` refuses catalog URLs on michaellanemusic.de before downloading anything.
- Left alone: `donate.rs` (PayPal link for ML Studio's author, opened by the user), and the publishing scripts and
  pages under `packaging/` (ML Studio's release tooling; not part of the app).

**Resolution of the new failures (2026-11-14, evening).** Each was checked against a clean ML Studio 14.2 tree
(`git archive 5fedeac2` copied into the build disk's `tree`, same target dir):
- `rec_smart` defaults to true (ML Studio always records smart); the legacy-writer test in `record.rs` turns it off
  under a `Drop` guard that puts it back.
- params / remote (AQUILA-only tests): the smart recorder writes its clip when the pass ends, so the asserts now
  follow Stop. `smart_record_value` again ignores a non-finite value before Start on input (AQUILA's order).
- record_lead, Melli archive: pass once smart recording is the default.
- engine `touch_override…`: the automation channel gets the full 0..1 range, as AQUILA scales clips by it (FL).
- mixer `toggle_slot`: back to ML Studio's code (`Snap::SlotEnabled`, "Effect bypass"); the control-plane test
  follows that name.
- drops: `add_audio_clip_in` routes an audio-mode lane through its linked output, as ML Studio 14.2 does.
- menus test lists Melli's label among those dispatched by label; two guide "Related" links say AQUILA.
- Baseline (`work/base-sorted.txt`, fail on clean ML Studio 14.2 too): os-theme `design::classic_is_unchanged`,
  `fl::the_light_mode_key_is_ignored`; `app::pressed_button_look`, `app::skins_and_themes_are_chosen_apart`,
  `app::playlist_floating_frame_has_no_inset_rail_and_panels_have_flush_edges`,
  `attached_mixer::ordinary_playlist_audio_recording_uses_clicked_lane_and_retains_binding`,
  `playlist_links::audio_import_uses_destination_only_while_attached`.
- Flaky only in the full parallel run (pass alone, here and in clean 14.2): `app::tempo_tapper_flash_repaints_the_pad`
  (global update status changed by other tests repaints the whole frame), `helper_install::melli_archive_validation…`.
- Result: `newfail.sh` shows the 17 of section 14 plus these two flaky tests.

**Order to finish:** commit the merge with only the merge files staged (`app.rs`, `updates.rs`, `feedback.rs`,
`vocal/engine.rs`, `helper_install.rs`, `HANDOFF.md` must stay unstaged for it), check and test again, commit the
second change, set the `ade` remote to https://github.com/ajdohaxhia/AQUILA.git, push both branches, read CI, then
run `aquila-release.yml`.

## 16. ML Studio v2026.11.14.3 merged (2026-11-14)

- Conflicts: `history.rs` keeps both rescale arms; `engine.rs` keeps AQUILA's `skipped` slot result plus 14.3's
  `record_delay`; `playlist.rs` takes 14.3's loop fitting (`add_audio_clip_fitted`, threaded through
  `add_audio_clip_in_fit`) with AQUILA's "Arranger edit" step; `updates.rs` and the NSIS installer keep AQUILA's
  text with version 2026.11.14.3; the guide and `github-release.json` take 14.3's text, named AQUILA. The new Helper
  visibility row reads "AQUILA Helper companion".
- Twelve UI tests new in 14.3 fail on a clean ML Studio 14.3 tree too and join the baseline: mixer layout
  (vintage seams, modern numbers, selected outline, separator, routing deck, traditional readouts, channel layout),
  menus (titles on first press, wheel rows, wide menus, nested plugin folders) and `recent::file_menu_section`.
- `newfail.sh`: the 17 of section 14 plus the two parallel-only flaky tests of section 15.

## 17. ML Studio v2026.11.16.4 merge — IN PROGRESS, paused for Codex (2026-11-16)

### State
- Branch `michael-lane`. Merge is **open**: `git merge --no-ff --no-commit origin/michael-lane` (MERGE_HEAD = 4da9381d, ML 15.1→16.4). Lemon `upstream/main` has nothing new.
- **Do not `git checkout`/`reset`/`merge --abort`** — about 35 conflicted files are already resolved by hand in the working tree.
- Last pushed/released AQUILA: ac430960, tag `aquila-v2026.11.14.3` (CI green on 3 OS).
- Version is already set to **2026.11.16.4**: packaging/github-release.json (tag v2026.11.16.4, title "AQUILA 2026.11.16.4"; `latest` is false, inherited from ML — set it to **true**), NSIS 2026.11.16.4, updates.rs VERSION 2026111604.

### Still conflicted (file: number of hunks)
```
crates/os-app/src/app.rs 20      crates/os-app/src/mixer.rs 11
crates/os-app/src/playlist.rs 9  crates/os-app/assets/native-effects-groups.json 8
THIRD-PARTY-NOTICES.txt 7        crates/os-app/src/prefs.rs 6
docs/user-guide.md 5             crates/os-app/src/history.rs 3
crates/os-app/src/views/menus.rs 3  views/playlist.rs 3  views/settings.rs 3
crates/os-engine/src/engine.rs 3 crates/os-engine/src/engine/pdc.rs 2
crates/os-app/src/plugin_menu.rs 2  plugin_window/mod.rs 2  project_plugins.rs 2
crates/os-app/assets/native-generators-groups.json 2   Cargo.lock 2
```
List them with `git diff --name-only --diff-filter=U` and `grep -c '^<<<<<<< '`. Markers are diff3 style (`<<<<<<<` AQUILA / `|||||||` base / `=======` / `>>>>>>>` ML).

### Resolution rules (applied so far, keep applying)
- Default: take ML's new logic/features, keep AQUILA names, labels, branding and AQUILA-only features.
- Text "ML Studio" → "AQUILA" in UI strings; keep author credits and license texts; keep donate.rs PayPal link.
- **AQUILA must never contact michaellanemusic.de.** updates.rs is already resolved: no network, manual check only shows the GitHub releases URL. feedback.rs keeps the GitHub issue link (not ML's Discord).
- If an AQUILA-only test breaks because of ML changes, fix the test, not shared ML code.
- Hints for open files:
  - **prefs.rs**: AQUILA keeps ROUTE_EDGE_SCROLL, ROUTE_RACK_SAMPLES, TEST_CHECK and its labels ("Select linked controls automatically", "Name channels after their sample or plugin", "Use the high-performance power plan"); take ML's other additions. AUTO_CHECK_UPDATES/AUTO_DOWNLOAD_UPDATES appear twice (≈ lines 304/308 and 675/677) — deduplicate.
  - **history.rs**: h1 needs **both** AQUILA's `RackSide` and ML's `Rows` (with new `ara_links` field). h2/h3: keep AQUILA's `track_io_of`, adopt ML's new `route: Option<(u16,bool)>` parameter on `add_channel`/`insert_channel` (uses `crate::sample_route::with_route`); keep replace_channel/timebase/project_info/project_settings wherever AQUILA has them. plugin_menu.rs calls must match (ML side passes `route`).
  - JSON group files / THIRD-PARTY-NOTICES / user-guide: union of both sides, AQUILA naming.
  - Cargo.lock: take either side, then let cargo regenerate.

### After all conflicts are resolved
1. `grep -rn '^<<<<<<<\|^>>>>>>>\|^=======$' crates plugins packaging docs` must be empty.
2. Build check on the external disk (needs `/Volumes/aquilabuild` mounted from `"/Volumes/SanDisk EXTREME SSD/aquila-build/aquilabuild.sparseimage"`):
   `./chk.sh check --workspace --lib --bins --tests` (or `chk-fast.sh` for incremental).
3. Tests: `work/newfail.sh --workspace --lib --bins --tests --no-fail-fast` → prints failures not in `work/base-sorted.txt` (log in work/last.log).
   Classify each new failure on a clean ML tree: `git archive 4da9381d | tar -x -C /Volumes/aquilabuild/work/ml164src`, rsync it into `/Volumes/aquilabuild/tree`, run the same cargo test with the same CARGO_TARGET_DIR. Fails on ML too → add to baseline with reason here; passes → fix AQUILA. Batch test names after `--`. tempo_tapper and Melli archive tests are flaky only in parallel.
4. Windows-only code can't compile on Mac: scan every plugin's host-link `match cmd` (cfg `plugin_windows`) for missing `os_hostlink::Cmd` variants, and Windows-only tests for new struct fields.
5. `grep -rn michaellanemusic crates/os-app/src` → only test refusals and the PayPal link allowed.
6. Commit "Merge ML Studio v2026.11.16.4 into AQUILA" (end with `Co-Authored-By:` line), **only after the user confirms**.
7. Push to `ade` (michael-lane and main), check CI (`gh run list -R ajdohaxhia/AQUILA`). Step "Test" has continue-on-error; exit-101 annotations on a green run are expected.
8. When green: `gh workflow run aquila-release.yml -R ajdohaxhia/AQUILA --ref michael-lane` → tag aquila-v2026.11.16.4. Update README version line.

### Do not
- `rm -rf` on /Volumes/aquilabuild (leave removals to the user; use new dirs).
- Generate or embed signing keys.

### Codex continuation (2026-11-16)
- Conflict markers resolved; workspace check passed on macOS. ML Plasma's native build was placed at `/Volumes/SanDisk EXTREME SSD/aquila-build/ml-plasma-merge-161604` because `/Volumes/aquilabuild` fell below its 6 GiB native-build preflight threshold. Set `ML_PLASMA_NATIVE_DIR` to that path when using the build scripts.
- Clean ML `4da9381d` full test comparison: seven `audio_edits::tests`, `warp::tests::legacy_tuned_warp_recovers_original_controls`, and `zipped::tests::audio_edits_portable_pending_revision_keeps_history_and_source` fail because the signed vocal tuning engine is absent. `helper_install::tests::melli_archive_validation_and_owned_extraction` and `app::tests::tempo_tapper_flash_repaints_the_pad` are parallel flakes. `app::tests::transport_clock_digits_share_a_bottom_line`, `mixer::tests::mixer_track_menu_renames_and_applies_custom_color_with_undo`, and `plugin_downloads::tests::catalog_browser_search_and_layout` also fail on clean ML in this environment. These 14 names were added to `/Volumes/aquilabuild/work/base-sorted.txt`; upstream log: `/Volumes/aquilabuild/work/ml164-full-tests.log`.
- Final full AQUILA run before the last fixes: 3,327 passed, 44 failed, 143 ignored. After comparing against clean ML and the existing baseline, the six remaining AQUILA-specific failures were addressed; focused reruns passed: 26 attached-mixer tests, 5 playlist header-drop tests, animation and fine-drag tests, and the small-effects layout test. A full suite was not rerun after those last focused fixes. The header-drop fix also adds a regression check that redo rehosts a dropped plug-in.
- Packaging Python tests: 27 passed. Windows host-link scan found no missing command variants; Windows-only struct literals were inspected but not compiled on Windows. Conflict-marker scan is clear. No merge commit or push has been made; section 17 requires user confirmation before committing.

## 18. ML Studio v2026.11.17.3 merged (2026-11-18)

- Merged ML 17.1–17.3 (1e1823b5). AQUILA version 2026.11.17.3 (`latest: true`). The 16.4 release was never published; this one supersedes it.
- 20 new ML plugins (`plugins/ml-*`) are linked in; their `Spec`s get AQUILA's `blank: None, locate: None`. New host-link `Cmd::StateEdit` = 17 (AQUILA shifted SampleFit to 16); every plugin `match cmd` has the arm, including AQUILA-only smith, lemon-slicer, lemon-drop.
- ML no longer publishes its factory template (`ML Mix Template.flp` is gitignored upstream). AQUILA keeps `Lemon Starter.flp` as the factory template; code, packaging and tests point to it, and only the old "Lemon Studio" category copy counts as legacy.
- Mixer: **Delete** keeps AQUILA's `delete_track` (refills the end, undo restores colours/icons, automation follows). ML's `delete_inserts` (mixer_delete.rs) runs for **Trim** only; its keyboard test is `#[ignore]`d with the reason. AQUILA test assertions that ML had rewritten to "Delete reduces the count" are restored.
- Engine: slide events carry both AQUILA's `colour` and ML's `slide_levels`.
- Piano roll follows ML's new gesture: Ctrl+Shift+click toggles, Ctrl+click selects only that note. Warp Ctrl+wheel zoom eases in AQUILA (test switches animations Off).
- Build: the `aquilabuild.sparseimage` no longer mounts. Paths with spaces (the SSD, Xcode on the SSD) break autoconf/CMake, so build in a copy on the internal disk:
  `rsync -a --exclude .git --exclude target ./ /Users/ade/aqbuild/tree/` then in that folder
  `DEVELOPER_DIR=/Library/Developer/CommandLineTools CMAKE_GENERATOR=Ninja PYTHON=python3 CARGO_TARGET_DIR=/Users/ade/aqbuild/target cargo test --workspace --lib --bins --tests --no-fail-fast`.
- Tests on macOS: 3446 pass, 56 fail. 44 also fail on clean ML 17.3 (`/Users/ade/aqbuild/mlfails.txt`); the other 12 already failed before this merge (add_audio_and_automation_clip_channels, browser_commands_jump_into_the_browser, playlist_time_markers, settings_window, tempo_tapper_window, toolbar_buttons_left_and_right, optional_parameters_tab…, slot_plugin_parameters…, declarations_match_the_settings_pages, lemon-small-fx menus_offer_reset… and the_effects_are_described_whole, linked_effects_save_wrapped_and_reopen).

## 19. Lemon Studio upstream merged (2026-11-18, upstream/main 6dfa0b3e)

- Taken: recording fixes (edits made during a take stay; a growing take clip leaves a taken track), `leave_project` (opening/New forgets the old project's interface state; AQUILA's smart cancel, mixer layout, harmony and references are kept in it), clips of every arrangement follow pattern moves, event editor / container / palette / "most recent" parameters follow moves, channel colour `for_channel`, plugin hold timing and the 0.1 ms parameter read, default recording input fixes.
- Not taken: Lemon's multi-output plugins (b42b1d8c). AQUILA already has ML Studio's multi-output design (`extra_outputs`/`extra_output`, instrument workers, `aux_pad`); the two are incompatible. multicore.rs stays deleted.
- Not taken: Lemon's solo-by-track-id (88c73d7d); AQUILA's solo logic (locked tracks, `audible`) stays index-based.
- The split workflow no longer remaps inactive arrangements by hand, since `renumber` now covers every arrangement.
- Tests: no new failures compared with section 18 (`added_mixer_tracks_save_and_reopen` fixed).

## 20. State at handoff (2026-11-24)

### Where things are
- Repo: this folder (SSD), branch `michael-lane`, clean. HEAD 2372e959, pushed to `ade` (github.com/ajdohaxhia/AQUILA) as both `michael-lane` and `main`.
- Commits since the 16.4 work: cf183e30 (merge ML 17.3), de84212e (merge Lemon), 7013321a (README + release script), 28e66d82 (Windows ChannelInfo `icon` fix), c08ee485 / 2372e959 (script robustness, Linux release on Ubuntu 24.04).
- CI 44257327611 on 28e66d82: green on Windows, Linux and macOS. 2372e959 changed only workflow/README/script.

### Release aquila-v2026.11.17.3 — INCOMPLETE
- Published with Windows zip, Linux tar.gz and SHA256SUMS. **macOS Apple silicon and Intel are missing**: in both release runs (44355216614, 44367306285) the macOS jobs were cancelled while waiting for a GitHub runner (no logs, capacity problem, not code).
- To finish: re-run only the failed jobs, which also re-runs Publish (it uploads with `--clobber` and rewrites the notes):
  `gh run rerun 44367306285 --failed -R ajdohaxhia/AQUILA`
  or a fresh `gh workflow run aquila-release.yml -R ajdohaxhia/AQUILA --ref michael-lane`. Check afterwards that the release lists 4 files plus SHA256SUMS.
- The Linux build now needs glibc 2.39 (Ubuntu 24.04+), because GCC 11 on 22.04 rejects ML's NAM engine C++. README says so.

### Parents
- **ML Studio: `origin` (github.com/Dorfhaus/ML-Studio) returns "Repository not found"** as of 2026-11-24 (deleted, renamed or made private). Last merged: 1e1823b5 = ML 2026.11.17.3. Find out where ML moved before the next sync; the full history up to 17.3 is already in this repo.
- **Lemon Studio: `upstream/main` has 94 new commits** not merged yet. Merge as in section 19; Lemon's multi-output and solo-by-id remain incompatible with AQUILA's ML-based design and are left out (revert those parts, keep the rest).

### Tools
- `packaging/release-when-green.sh [commit] [version]`: waits for that commit's CI run, starts aquila-release.yml only if green. Keeps the Mac awake (`caffeinate`) and survives network errors. Log it with `nohup ... > /Users/ade/aqbuild/release-when-green.log 2>&1 &`.
- Local build/tests: copy to `/Users/ade/aqbuild/tree` (section 18 command). `/Users/ade/aqbuild/mlfails.txt` = failures on clean ML 17.3; `fails3.txt` = AQUILA's after the Lemon merge (56, none caused by merges). `/Users/ade/aqbuild/ml173` and `pre` are scratch trees and can be deleted.
- Windows-only code can't be compiled on the Mac: after a merge, grep struct literals (`ChannelInfo {`, `NoteLevels {`, `SlotPlugin {`, `Spec {`) and every plugin `match cmd` for new fields/variants before pushing.
- The ML-inherited workflow `release.yml` ("Release", scheduled) always shows "skipped" in AQUILA; it is harmless and can be disabled.

### Rules (unchanged)
- No contact with michaellanemusic.de; keep all credits, licenses and the donate.rs PayPal link; no signing keys.
- Commit, push and release only after the user confirms.
