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


## 21. Lemon synchronization and supplied ML sources (2026-11-24, in progress)

- Owner authorized conflict resolution, commit and push to AQUILA.
- Working merge targets Lemon `ce48717e` first (62 reachable commits after the previously merged `6dfa0b3e`). The later control-plane/session split through `349c6ba8` still needs integration. Do not report full Lemon parity until that is complete.
- Preserved AQUILA's ML engine/multi-output design, index solo, UI, extension and update-server guards. New SoundFont player, Windows 32-bit VST2 bridge, plugin edit notifications, import/audio fixes, tempo menu, browser waveform metadata and clip brush are being integrated. Host-link `Edited` is 18, keeping BrowseSample/SampleFit/StateEdit at 15/16/17.
- Sources supplied by owner remain in Downloads. `ML Studio Source.zip` and `ML Studio Source (1).zip` have identical content. Both Mac archives also have identical content. Ignoring CRLF, only five paths differ between groups. Generic archives have maintenance build 2026112002, Mac archives 2026112402. The Mac source changes include plugin input-queue priority and its regression test, plugin window checks, update version, and omission of the visualizer worker source ZIP.
- Mac source extracted separately into `/Users/ade/aqbuild/ml-source-2026112402` for comparison. These new ML sources have not been merged into AQUILA yet.
- Current verification copy is `/Users/ade/aqbuild/lemon-sync`. Native Plasma cache is `/Users/ade/aqbuild/lemon-plasma-native`. Logs are `lemon-check.log`, `lemon-core.log`, `lemon-half.log` in /Users/ade/aqbuild. Plugin checks in `plugin-sync.h7oU9K/tests.log` passed 118 tests (one ignored).


### Verified pre-control-plane integration

- `cargo check --locked -p os-app --tests` passed on Apple silicon with native ML Plasma built from source.
- Engine tests passed 464, failed 0, ignored 11. Host-link tests passed 4. SoundFont/Sampler/Press tests passed 118, failed 0, ignored 1. Notices tests rerun after completing missing dependency licences.
- Upstream device playback test requires explicit `OS_SIM_DEVICE=256@48000` and is ignored in the normal parallel suite. Windows i686 bridge test is wired into CI. Windows/Linux runtime verification is pending.
- Supplied newer ML sources change at least 303 paths already tracked in the last merged ML 17.3 base. They need a separate merge after the Lemon architecture work.

## 22. Lemon control-plane merge paused at owner's request (2026-11-25)

### Git and working tree

- Checkout: this repository, branch `michael-lane`. HEAD `176cedea260c336cfcebdcc0180610afe5a06d49` is published on both `ade/main` and `ade/michael-lane`.
- A merge of `upstream/main` `349c6ba832c683c5e7bbbdf06edcf5e400efdcfe` is **in progress**. `MERGE_HEAD` is present. Git reports no unmerged entries, but this only means paths were staged; the integration is incomplete and does not compile. Do not commit or push it as complete.
- Approximately 681 paths are modified/staged. Preserve both staged and unstaged work, including untracked `crates/os-session/src/ml_patch.rs`. Inspect `git status --short`, `git diff`, `git diff --cached`, and `git diff --cached --check` before resuming. Several files have trailing blank-line warnings.
- Work stopped during edits. In particular `crates/os-app/src/lib.rs` has a standalone `pub` at approximately line 165, an incomplete edit that must be repaired. `crates/os-engine/src/engine.rs`, `crates/os-engine/src/engine/audit_tests.rs`, `crates/os-session/src/locked.rs`, and `crates/os-session/src/session.rs` have unstaged changes. Review and preserve them.
- Owner requested stop and handoff. No commit or push was made for this second merge.

### What was integrated so far

- New `os-session` crate and `lemonstudio` launcher introduced, while retaining AQUILA's desktop shell and ML-based multi-output, solo and custom plugins.
- `State` owns one shared `Session`; a project-only session has a disabled transport for GUI, while AQUILA's existing GUI transport owns audio. `follow.rs` contains an unverified `App::host` adapter intended to synchronize command edits, plugin hosts and GUI transport. Session direct project writes have a compatibility `DerefMut` which marks writes.
- Desktop undo was adapted around a shared session history with GUI snapshot handling. Desktop RPC undo/redo defers through GUI history after session authorization. Plugin editor and touch-note calls were partially migrated to `EditorHost` and GUI transport.
- Session capture/monitor/pickup statics were moved toward shared ownership with GUI shims. Verify recording and ASIO input device selection, and ensure there is only one set of capture rings.
- Lemon's arbitrary extra output routing conflicts with AQUILA's engine. Session rejects the incompatible `SetChannel` outputs operation rather than silently claiming it worked. Preserve AQUILA's consecutive extra-output implementation, and validate normal plugin routing.
- UI conflict resolution retained AQUILA's legacy UI modules; Lemon's new `*_ui` sources were retained but many are not currently compiled. The command-based UI migration and full parity therefore remain outstanding.
- The prior published CI run `45469126783` failed on Windows because `os-vst2::abi::op::OUTPUT_PROPERTIES` was missing. A staged fix adds `0x22`; Linux was green, macOS was cancelled. Windows needs a fresh build.

### Verification and continuation

- Before the latest interrupted edits, `cargo check -p os-session --offline` passed with default features (log `/Users/ade/aqbuild/split-default5.log`) and `cargo check -p os-session --no-default-features --offline` passed (log `/Users/ade/aqbuild/split-check4.log`). Both must be rerun on the current tree.
- The last app check against a scratch copy failed with 314 errors (log `/Users/ade/aqbuild/split-app.log`). Subsequent edits addressed some errors, but no later full app check completed. Expect missing module exports, plugin editor APIs, State/Session borrowing and history/transport interface mismatches.
- Build from a copy without spaces. Sync into `/Users/ade/aqbuild/split-check`, then set `DEVELOPER_DIR=/Library/Developer/CommandLineTools PYTHON=python3 ML_PLASMA_BUNDLE_MACOS_AARCH64=/Users/ade/aqbuild/target/debug/build/ml-plasma-f7022116b0af3b77/out/plasma.zip CARGO_TARGET_DIR=/Users/ade/aqbuild/target`. Run `cargo check -p os-session --offline`, the no-default-features check, then `cargo check -p os-app --offline`. The bundled Plasma zip is an existing local artifact; a fresh native build can instead use a new space-free `ML_PLASMA_NATIVE_DIR`.
- Agent findings to verify: plugin windows, presets and Session host method signatures remain incomplete; `follow.rs` previously referenced `params_ui` and `container_ui` which were absent from `lib.rs`; GUI Session commands and live playback must reach the GUI transport; restored UI modules still need command migration. Do not mark parity by merely recording Lemon as a merge parent.
- Supplied ML source archives remain in Downloads. The Mac archive was extracted to `/Users/ade/aqbuild/ml-source-2026112402`; its changes from the previously merged ML 17.3 base have not been merged. See section 21.
- After compilation, run meaningful session, GUI, host-link, plugin and audio tests, check Windows-specific struct literals and plugin command matches, review third-party notices, verify no contact with michaellanemusic.de, then commit and push to both AQUILA branches as the owner previously authorized.

## 23. Lemon control-plane merge: continuation 2026-11-25

### State
- The Lemon merge of `upstream/main` `349c6ba8` is still **open** (`MERGE_HEAD` present, ~682 staged paths, nothing committed). The tree compiles as far as **`os-session` (clean, 0 errors)**; **`os-app` still has 196 errors**. The owner asked to finish this merge and only then merge ML Studio's new sources (see below).
- ML Studio sources 2026.11.24.2 are in `/Users/ade/Downloads/ML-source` (four ZIPs, no Git repo; Mac archive VERSION `2026112402`). Already extracted at `/Users/ade/aqbuild/ml-source-2026112402`. **Not merged yet** — per the owner's decision it comes after the Lemon merge. A three-way merge is needed: base = the ML 17.3 tree already in this repo (`1e1823b5`), theirs = the 2026.11.24.2 snapshot, ours = AQUILA.

### Environment for checks (verified this session)
```
rsync -a --delete --exclude .git --exclude target ./ /Users/ade/aqbuild/split-check/
cd /Users/ade/aqbuild/split-check
export DEVELOPER_DIR=/Library/Developer/CommandLineTools PYTHON=python3
export ML_PLASMA_BUNDLE_MACOS_AARCH64=/Users/ade/aqbuild/target/debug/build/ml-plasma-f7022116b0af3b77/out/plasma.zip
export CARGO_TARGET_DIR=/Users/ade/aqbuild/target
cargo check -p os-session --offline   # passes
cargo check -p os-app --offline       # 196 errors
```
The bundled Plasma zip is **required**: without it the build script runs CMake against a stale cache and fails. Full logs: `/Users/ade/aqbuild/split-app-short.log` (short format), `/Users/ade/aqbuild/split-app2.log`.

### Fixed this session
- `crates/os-app/src/winit_keys.rs`: added from upstream's `crates/lemonstudio/src/winit_keys.rs` (`use crate::keys`), `pub mod winit_keys;` in `lib.rs`.
- `shell.rs`: `pub(crate) use macos::{… screen_state …}` so `lib.rs`'s re-export works.
- `settings_file.rs`: added `store()` (memory in tests, user file otherwise) for the session APIs.
- `lib.rs` wiring: `osc3/citrus/smith/slicer/drums` now `pub use os_session::…` + their `*_ui` modules; added `song_pos_ui`, `score_log_ui`, `clip_colors_ui`, `plugin_scan_ui`. `crate::song_pos::`/`crate::score_log::` call sites now point at the `_ui` modules.
- `State`: added `rack_views: HashMap<u64, follow::RackView>` (upstream field dropped in the merge); `follow.rs` uses `s.params.remap`/`s.params.last` instead of upstream's separate `tweaks` vector, `rack::remap_ui` instead of `remap_view`, `app::retime` instead of `retime_views`, and `container::show_level` (added to `container.rs`).
- `os-session::plugins::NO_LIBRARY` is now `pub`; `crates/os-app/src/plugins.rs` re-exports it.
- `Channel` literals: `lemon_icon: None` added in `app.rs`, `pattern_render.rs`, `playlist.rs`. `TimeMarker`/`AutoPoint` literals: `id: Default::default()`.
- **Do not compile `params_ui.rs` or `container_ui.rs`**: they are Lemon's UI and duplicate AQUILA's `params.rs`/`container.rs` (`E0592`, ambiguous names). AQUILA's legacy modules stay.

### Remaining (in order)
1. Plugin-host UI moved out of `Plugins` into `State.editors` (`EditorHost`): `row_at_screen`, `drop_file`, `cancel_sampler_track_step`, `close_if_open`, `open_window`, `set_detached`, `set_all_detached`, `set_editor_scale`, `rescale_window`, `refresh_known`, `take_known`, `stack`, `clip`, `idle`, `editor_open` — each call site must pass `&mut state.session.plugins`. `Hosted.window`/`rack_editor`/`detached`/`pending_editor_resize` became `EditorHost` state (`window_of`, `set_detached`, …).
2. `preview_jobs` (preset previews) has no home now: move it to `EditorHost` or `App`.
3. `plugins::browser_sample_to_plugin`, `plugins::load_slicer_loop` (session now offers `slicer_loop_state` + `set_state`), `plugins::fit_editor` (now `editor_host::fit_editor`), `mixer::track_ids`, `control::accept` (now `control_ui::accept`, done in `app.rs` only if it still compiles).
4. Duplicate-type seams: `crate::history::Snap` vs `os_session::history::Snap`, `crate::typein::Target` vs `os_session::typein::Target`, `container::Live`, `lemon_midi_out::Live` — prefer re-exporting the session type from the legacy module.
5. ~60 borrow-checker errors (`E0499`/`E0502`/`E0503`) from `State`'s `Deref` to `Session`: take a field out before calling, or borrow `session.plugins` once.
6. `record::Log`/`DUMPED` are private in `os-session::record`; make them `pub`.
7. Then: full workspace tests, Windows-only struct/`match cmd` scan, no michaellanemusic.de contact check, commit (owner confirmation), push, CI.

### 23.3 Aggiornamento: prima passata di `cargo test --workspace`

`cargo test --workspace --locked --offline --no-fail-fast` sulla copia `/Users/ade/aqbuild/split-check`: **139 test falliti** su 7 target (`lemon-small-fx` 3, `ml-mix-core` 4, `ml-utility-core` 1, `os-app` ~100, `os-session` 5, `os-theme` 1, `watermark` 1). Log: `/Users/ade/aqbuild/test-ws2.log`.

Corretti e riverificati:
- `lemon-small-fx`: nomi "ML …"/"Lemon …" accettati entrambi; i factory preset si cercano in `crates/os-app/factory-presets` e `crates/os-session/factory-presets`; menu About con il nome dello spec. Ora **63 passati, 0 falliti**.
- `ml-utility-core` (7 passati) e 3 di `ml-mix-core`: `ML_TOOLS_RENDER_DIR` cade su `std::env::temp_dir()` quando non è impostata. Resta `reference_match_reload_missing_and_callback_safety`: **una deallocazione nel callback audio** quando adotta una nuova reference (difetto reale, non corretto).
- Helper rack (`rack.rs`): `add_group`/`zip_rows`/`lock` specchiano progetto e lista mostrata; `set_mixer_track` passa da `control::run_edit`; `RBox::GroupSel/NewGroup/RenameGroup` ora scrivono i gruppi nel `Project` (`sync_groups`); `RCmd::Solo` registra un passo `MUTE_SOLO`, `GroupSel` no; `Cmd::PerfToggle` non registra più un passo. `control::tests::a_filter_group_and_performance_mode_are_no_step` passa.

Restano da portare (il grosso): i test os-app leggono dal `Project` stato che l'app AQUILA tiene in `State.mixer`/`State.rack`/UI (colori slot, gruppi, performance), quindi edit e undo non si vedono; molti comandi registrano passi che Lemon considerava di sola interfaccia; `os-theme::design::classic_is_unchanged` e i test skins/temi hanno la palette AQUILA vs quella del merge; guide Help senza i topic nuovi ("Visibility: ML Patch…"); nomi plugin/progetti misti ML/Lemon; 7 test `audio_edits`/`warp`/`zipped` richiedono il motore nativo "Vocal tuning"; `factory_folder` richiede FL Studio installato; `ai_control` fallisce sul parsing della status line HTTP locale.

### 23.2 Aggiornamento: anche `--all-targets` a zero errori

`cargo check --workspace --all-targets --locked --offline` esce con **exit 0, zero errori** (stessa copia `/Users/ade/aqbuild/split-check`); restano solo warning. I test sono stati **compilati, non eseguiti**.

- `crates/os-app/Cargo.toml`: `[dev-dependencies] os-session = { workspace = true, features = ["test-util"] }`. Le suite di os-app usano gli helper della sessione (`History::len`, `plugin_db::fake`, `capture::TEST_RECORDING`, `Transport::updates_for_tests`, `time::advance/set_clock`, ...).
- os-session: `capture::push_inputs` è disponibile con `test-util` (era `cfg(any(windows, test))`); `input_monitor::Monitor::render` passa a `cfg(any(test, feature = "test-util"))` e nasce `render_owned_for_tests(owner)` per le suite esterne.
- Suite Lemon riportate sull'architettura AQUILA: `Locked::new(...)` nelle 234 assegnazioni a `state.project`; `App.editors`/`EditorHost` al posto dei campi rimossi di `Plugins`/`Hosted` (`detached`, `rack_error`, `link`, `refresh_known`, `editor_open`); `vocal_edits` e `channels` letti dal `Project`; `History::entries(&SettingsStore::memory())`; `prefs`/`midi_in`/`transport`/`step_edit` presi da `os_session`; `crate::{locked, projects, midi_notes, project_io, knob_id}` ri-esportati in `lib.rs`; `presets`/`perform`/`song_pos_ui` al posto di `presets_ui`/`perform_ui`/`song_pos`; helper rack e `mixer::slot_names_of` aggiunti/ri-esportati.

### 23.1 Aggiornamento: 262 → 0 errori

Merge Lemon **ancora aperto** (`MERGE_HEAD` = `349c6ba8`), nulla committato/pushato. `cargo check -p os-app --offline` esce con **exit 0, zero errori** (verificato sulla copia di build `/Users/ade/aqbuild/split-check`, sorgenti identici).

**Porting finale**: `plugins/window_check.rs` usa ora `EditorHost` (`idle`, `set_detached`, `open_window`, `rescale_window`, `editors[row].window`); rimosso il blocco `pending_editor_resize` (la coda non esiste più nel nuovo `Frame`). `plugin_window::new_frame(...)` è il nuovo costruttore che incapsula il `FrameWindow` di piattaforma in `Box<dyn Frame>` (usato da `ml_patch`). `typein`: `knob_to_session`/`knob_from_session` mappano i `KnobId` condivisi; `Varispeed`, `ChanPitch`, `Strip` e i knob dei dialoghi Strum/Arp/PianoDlg non hanno equivalente di sessione (ritornano `None`/fallback).

**Nota**: il merge **non è committato**. Prossimi passi: `cargo check` dell'intero workspace, scan dei path Windows-only, controllo che nulla contatti michaellanemusic.de, poi conferma dell'owner prima di commit/push e del merge ML 2026.11.24.2.

**Scoperta chiave**: `Session.project` è `Locked<Project>`. Per separare i campi bisogna dereferenziare una volta: `let p = &mut *s.session.project;` (non `&mut s.session.project`, che presta tutto il `Locked`). Stessa cosa per `&*s.session.project` nelle letture.

**Regola d'oro per i borrow** (`State` fa `Deref` a `Session`, quindi ogni accesso via `s.campo` presta TUTTO `*s`):
- Campi di `Session` (prefissarli `s.session.` per farli separare dal resto): `project settings history control file path dirty sample_loads retimed plugin_edited loop_type loop_type_of touched samples fx_requests link_moves plugin_members plugins hosted_rows pending_plugin pending_open params midi midi_play midi_channel midi_out rpc_server loading missing zipping export pattern_render saved backup perform live_synced clip_colors song_pos pat_pos ext_pending ext_tempo_at rec audio_rec track_rec transport`.
- Tutti gli altri (`playlist rack mixer pattern tempo piano plugdb container palette events settings-ui …`) sono campi **di `State`**: restano `s.campo` e si separano da `s.session.*`.
- Esempio: `let p = &mut s.session.project;` + `s.playlist…` funziona; `let p = &mut s.project;` + `s.playlist…` no.

Completato:
- **EditorHost**: ogni call site di finestra plugin passa da `Plugins` a `EditorHost`, che vive su **`App.editors`** (`State` non ha `editors`). `strip_editor*.rs` espongono già `show_rack`/`close_rack`/`close_racks`/`validate_vertical_rack` con `pl: &mut Plugins`.
- `preview_jobs` su `EditorHost`; `browser_sample_to_plugin` aggiunto a `EditorHost`; `load_slicer_loop` riscritto con `slicer_loop_state` + `set_state`; `mixer::track_ids` era una chiamata morta, rimossa.
- Seam: `lemon_midi_out` → re-export della sessione (+ `mod lemon_midi_out_ui` in `lib.rs`); `container::Live` → re-export (`src` reso `pub` in os-session); `presets::Preset` → re-export; `history::Snap`: arm `Snap::Session` duplicato rimosso, `put` delega alla sessione; `control_ui` usa `crate::history::record`.
- os-session: `Plugins::prepare_preset`/`install_preset`; `Src::Mpe`; `input_monitor::Monitor.track` pub.
- `rack.rs`: `pattern.loop_off` → `pattern.loop_mode`.
- `plugins.added` prende lo store: `&state.session.settings` (`state.settings` è `SettingsUi`).
- `note_name`/`key_hint`/`keep_long`/`more_presets`/`History::entries` prendono `&crate::settings_file::store()`; `ticks_per_second` in `score_log_ui`; `ClipColors::of` in `clip_colors_ui`; `PANEL` in `song_pos_ui`; `Locked` in `os_session::locked`.
- Conversioni `typein::Target` ↔ `os_session::typein::Target` (`to_session`/`from_session`), usate da `follow.rs` e `container.rs`.

Restano:
1. **~96 errori di borrow** (`E0499/E0502/E0503`) dal `Deref` su `State`/`App`: scrivere i campi per esteso (`s.session.project` + `s.session.pattern`, `app.state.project` + `app.state.session.plugins`) o tirare fuori il valore in un `let` prima della chiamata mutabile. File: `mixer.rs` (13), `record.rs` (7), `rack.rs` (6), `app.rs` (6), `playlist.rs` (5), `audio_edits.rs` (5).
2. **`KnobId` duplicato** (`crate::knobs::KnobId` con impl UI vs `os_session::knob_id::KnobId`): `typein::to_session`/`from_session` falliscono solo su `Target::Knob`.
3. `plugins/window_check.rs` (7): riscrivere su `EditorHost` (`frames`, `open_window`, `rescale_window`, `close_if_open`).
4. `ml_patch/mod.rs`: `FrameWindow::new` non esiste più (aprire via `Frames`).
5. `editor_host.rs`: `Plugin::take_playback_request` rimosso/rinominato in `os_host`.
6. `app.rs:3213` e `app.rs:8028`: due chiamate con arità cambiata.

Verifica: `rsync -a --delete --exclude .git --exclude target ./ /Users/ade/aqbuild/split-check/`, poi in quella cartella `cargo check -p os-app --offline --message-format=short` con `DEVELOPER_DIR=/Library/Developer/CommandLineTools`, `PYTHON=python3`, `ML_PLASMA_BUNDLE_MACOS_AARCH64=/Users/ade/aqbuild/target/debug/build/ml-plasma-f7022116b0af3b77/out/plasma.zip`, `CARGO_TARGET_DIR=/Users/ade/aqbuild/target`.


## 24. Merge Lemon upstream/main 0720e512 in AQUILA: fix dei test e stato CI (2026-11-26)

Branch `michael-lane`, remote `ade` (`github.com/ajdohaxhia/AQUILA`; `origin`/Dorfhaus non esiste piu'). La CI parte solo con `workflow_dispatch` (o su `main`), quindi dopo ogni push va rilanciata a mano.

Commit:
- `9112c03d`: merge di `0720e512` (genitore `176cedea`), pushato.
- `021c06d2`: `ml_patch` e `winit_keys`, dichiarati in `lib.rs` ma mai committati (E0583 su un checkout pulito, CI macOS).
- `0449b8ec`: i due fix dei test control (sotto).
- Il commit di questo aggiornamento: nome del Trio, attese dei preset e questo file.

### 24.1 Cosa e' stato corretto

1. `control::tests::edits_in_a_plugin_s_window_are_undo_steps`: AQUILA tiene la cache dei parametri nell'app (`State.params`, `params_tick` in `crates/os-app/src/params.rs`), mentre undo/redo li impostavano solo via sessione (`Session.params`, mai letta dall'app). Dopo un undo il plugin tornava a 0.25 ma la cache restava a 0.7, e il gesto successivo registrava 0.7 come valore di partenza. `App::host_commands` (`crates/os-app/src/control/host_ui.rs`) ora riporta nella cache dell'app ogni valore che l'host ha davvero impostato (verificato con `Plugins::param_value(key, param) == v`, cosi' un plugin che rifiuta il valore non sporca la cache).
2. `control::tests::preset_loads_are_undo_steps`: `presets::auto_name` scriveva ancora la mappa ritirata `State.mixer.slot_names`, mentre il nome dello slot vive nel modello (`edit::mixer::slot_*`). Ora scrive `crate::edit::mixer::set_slot_label`, e `presets::side` legge `slot_label`; l'undo riporta il nome precedente ("Fruity Delay 3"/"ML Delay") e il redo "Bright".
3. Regressione vera del merge: `os_session::osc3::LEMON_TRIO` era rimasto `"Lemon Trio"` (upstream) mentre il plugin compilato si chiama `"ML Trio"` e `OLD_NAMES` registra gia' il rename (`"Lemon Trio" -> "ML Trio"`). `crates/os-app/src/lib.rs:164` fa `pub use os_session::osc3`, quindi **ogni** lookup del Trio nell'app (menu, `sync_osc3`, `project_plugins`, titoli) usava il nome sbagliato: `control::tests::a_replaced_generator_stays_replaced` non trovava il plugin. Costante portata a `"ML Trio"` (unica eccezione: le altre costanti stock - `ML Sampler`, `ML Citrus`, `Smith`, `Lemon Slicer`, ... - combaciano gia' con i descrittori).
4. `os-session` `presets::tests::old_built_in_folders_are_taken_over`: attese ferme ai nomi Lemon (`"Lemon Trio"`, `"Lemon Sampler"`) mentre le cartelle dei preset seguono il nome canonico del plugin (`folder_name(LEMON_SAMPLER)` = `"ML Sampler"`). Aggiornate a `"ML Trio"` e `"ML Sampler"`.

### 24.2 Verifiche (copia di build `/Users/ade/aqbuild/split-check`)

```
rsync -a --delete --exclude .git --exclude target "<progetto>/" /Users/ade/aqbuild/split-check/
export DEVELOPER_DIR=/Library/Developer/CommandLineTools PYTHON=python3 \
  ML_PLASMA_BUNDLE_MACOS_AARCH64=/Users/ade/aqbuild/target/debug/build/ml-plasma-f7022116b0af3b77/out/plasma.zip \
  CARGO_TARGET_DIR=/Users/ade/aqbuild/target
cargo test -p os-app --lib --locked --offline <filtro>
```

- `control::tests` (dopo `0449b8ec`): **64 passati, 2 falliti** (`ai_control` e `a_replaced_generator_stays_replaced`); prima dei fix erano 62/4.
- `control::tests::a_replaced_generator_stays_replaced`: **ok** dopo il fix del nome.
- `os-session` `osc3::` tests: **ok**; `presets::tests::old_built_in_folders_are_taken_over`: **ok** dopo l'aggiornamento delle attese.
- Log: `t-ctl13.log` (control), `t-trio.log` (trio/osc3/presets/project_plugins), `t-presets.log`.

### 24.3 Non regressioni e rosso residuo

- `ai_control::tests::ai_control_auth_approval_commands_and_revocation`: **non** e' una regressione del merge. Fallisce sulla prima richiesta HTTP locale di questo host macOS, nel client `ureq`: `Error encountered in the status line: Invalid argument (os error 22)`. Renderlo bloccante gli stream accettati (ereditano `O_NONBLOCK` dal listener su BSD/macOS) non cambia nulla: tentativo revertito. Era gia' classificato ambiente/stale in §14 ("`ai_control` (local network)").
- I sei `project_plugins::tests` rimasti rossi erano quattro regressioni vere del merge e due attese stale: tutti corretti, dettaglio in §24.5.

### 24.4 CI e release

- Run `45890018739` e `45893023913`: cancellate (la prima per E0583, la seconda superata dai fix).
- Run `45894457693`: lanciata su `0449b8ec`; `45893023913` era quella seguita dal watcher `/Users/ade/aqbuild/auto-release.sh`, ora cancellata, quindi nessuna release e' partita.
- Lo step `Test` della CI e' `continue-on-error: true`: Build/Bundle sono i passi che fanno fallire la run; i fallimenti noti non rendono rossa la CI.
- Release `aquila-release.yml` **non ancora lanciata**: va dispatchata con `-f version=2026.11.26.1 -f publish=true` solo dopo una CI verde sul commit che si vuole pubblicare (`gh workflow run aquila-release.yml --ref michael-lane -R ajdohaxhia/AQUILA ...`).

### 24.5 I sei `project_plugins::tests` rimasti rossi (2026-11-26)

Quattro regressioni vere del merge, due attese di test stale. Nessun test saltato e nessuna asserzione indebolita; le asserzioni su wrap/save/reopen restano quelle di prima.

1. `linked_effects_hall_visibility_keeps_host_available` e `linked_effects_tape_visibility_keeps_host_available` — **bug vero**. Le liste dei plugin leggono i flag `visible_plugin-hall` / `visible_plugin-tape` da `Plugins.settings`, ma `Session::new` costruiva `Plugins::default()` con uno store nuovo e scollegato: i flag erano ignorati finche' non partiva uno scan (`plugin_scan_ui` era l'unico punto che li collegava). `Session::new` ora passa lo stesso `SettingsStore` della sessione a `plugins` (`crates/os-session/src/session.rs`).
2. `ml_renames_restore_legacy_wrappers_without_substitution` — **bug vero** (rename). `plugins::slot_label` aveva perso la canonicalizzazione: uno slot o un progetto salvato col nome vecchio ("Lemon Sampler") restava vecchio invece di "ML Sampler". `slot_label` ora termina con `canonical_name` (`crates/os-session/src/plugins.rs`). La parte finale del test cercava il riverbero con `effect_index("Lemon Reverb")`, nome che nessun percorso di produzione usa (i riferimenti di progetto passano da `find`, che canonicalizza): la riga ora usa `"ML Reverb"` (attesa stale).
3. `project_saves_with_a_refusing_bus_effect_and_keeps_its_last_state` — **bug vero**. `Plugins::state` non rispettava piu' `state_fail` (l'hook che simula un plugin che rifiuta `save_state`): il salvataggio riusciva in silenzio invece di avvisare e conservare l'ultimo stato avvolto. Controllo ripristinato; e' lo stesso hook usato da `audit_fixes`.
4. `real_native_plugins_replace_the_open_recreations_and_back` — **bug vero**. `App::sync_osc3` (`crates/os-app/src/osc3_ui.rs`, il modulo vivo) chiamava `sync_osc3` senza la guardia `native_first()`, che esisteva solo in `crates/os-app/src/osc3.rs`, file non piu' compilato (`lib.rs` fa `pub use os_session::osc3`). Con i plugin nativi attivi la ricreazione del Trio tornava a essere ospitata. Guardia ripristinata nel metodo vivo.
5. `linked_effects_save_wrapped_and_reopen` — **attesa stale**. Il test prendeva il primo parametro editabile e pretendeva che il valore si spostasse: `ML Meter` non ha parametri editabili e il primo parametro di `ML Vocoder` ("Carrier") e' a passi, quindi 0.3 tornava al default 0.0. Ora il test prova i parametri editabili finche' uno si muove e salta gli effetti che non ne hanno.

Verifiche (stessa copia di build di §24.2, log `t-pp5.log`, `t-session.log`, `t-plugins.log`, `t-base.log`, `t-base2.log`):

- `cargo test -p os-app --lib project_plugins::`: **21 passati, 0 falliti**.
- `audit_fixes`: **ok**; `control::tests`: **65 passati, 1 fallito** (`ai_control`, §24.3).
- `os-session --lib`: **196 passati, 4 falliti**; filtro `plugin` di os-app: **105 passati, 4 falliti**. Gli otto rossi sono pre-esistenti: rifatti su un checkout pulito di `84834b9a` (`git archive HEAD` in `/Users/ade/aqbuild/base-check2`) falliscono identici (`ML Amp` senza categoria factory, `every_lemon_effect_is_built_in` senza `ML Patch`, fixture zip, frame di coda del transport, conteggi params, catalogo download, keyword help, cartelle factory), quindi non vengono da questi fix.
- File toccati: `crates/os-session/src/session.rs`, `crates/os-session/src/plugins.rs`, `crates/os-app/src/osc3_ui.rs`, `crates/os-app/src/project_plugins.rs`.

### 24.6 CI sui sei fix (commit `2e690c2b`, 2026-11-26)

- Push su `ade/michael-lane`: `2e690c2b` (i sei fix + §24.5).
- CI dispatchata a mano su `2e690c2b`: run `45903363010` — **rossa**: `Windows: Build`, `Linux: Build`, `macOS: Build the session alone`. (In parallelo il workflow "Check public source for Google API keys" sullo stesso push e' verde.)
- `macOS: Build the session alone` (`cargo check -p os-session --all-targets`) fallisce con `E0609: no field state_fail on type &Plugins` (`plugins.rs:1033`): la mia correzione del punto 3 usa `self.state_fail`, ma il campo e' `#[cfg(any(test, feature = "test-util"))]`. Fix locale **non committato**: la stessa `cfg` sulla guardia in `Plugins::state`. Con la `cfg`: `cargo check -p os-session --all-targets` ok e `project_plugins::tests::project_saves_with_a_refusing_bus_effect_and_keeps_its_last_state` ok (log `t-cfgfix.log`); il loop `--no-default-features` della CI non e' stato rifatto in locale (lo shadow build ha bisogno di `ML_PLASMA_BUNDLE_MACOS_AARCH64`), quindi va verificato con un nuovo commit + dispatch.
- `Linux: Build` e `Windows: Build` falliscono per le stesse cinque `E0*` gia' presenti sul commit precedente `84834b9a` (merge, non questi fix): `Hosted.window` (`browser.rs:553`), `Plugins::open_window`/`close_window` (`playlist.rs:6495`), `From<transport::Drops>` per `dropout_log::Drops` (`app.rs:2195`), `Option<Box<dyn Any>>` vs `Option<Unpinned>` (`app.rs:3100`). Sono percorsi di codice solo Linux/Windows (x11/windows) rimasti indietro rispetto a `os-session` dopo il merge: la CI restera' rossa finche' non vengono sistemati.
- Conseguenza: nessuna release lanciata (`aquila-release.yml` richiede CI verde).

### 24.7 Build Linux/Windows: cause e correzione (2026-11-26)

I due job Build non dipendono dai sei fix: sono API del merge rimaste indietro rispetto ai moduli upstream. Corretti tutti i punti.

**Windows (`Windows: Build`, os-session).** `crates/os-session/src/device.rs` e' quello nuovo di Lemon (`Mix::block(frames)` e `Mix::set_width(pairs)`, nessun `DeviceBlock`, nessun `playback_at`), ma `device/asio.rs` e `device/wasapi.rs` erano ancora le copie ML Studio sulla vecchia API (`prepare_outputs`, `block_at`, `Mix::headphones`, `MAX_OUTPUT_CHANNELS`). Portati alla nuova API:

- ASIO: tolto `headphones.set_rate` (la correzione cuffie vive nel layer di `os-app`, non in `os-session`); cap output `2 * MAX_OUTPUTS`; `mix.set_width(channels.div_ceil(2))`; il callback legge `mix.width` prima di `mix.block(frames)` e scrive i campioni interleaved (`i * width + ch`).
- WASAPI: tolto `headphones.set_rate` e `playback_at` (la nuova `Mix::block` non lo prende); `mix.set_width(s.channels.div_ceil(2))`; `write` prende la slice interleaved e la larghezza.
- Test dei due backend aggiornati alla `Mix` interleaved (stessi valori e stesse asserzioni di canale).

**Linux (`Linux: Build`, os-app).** Il merge ha spostato le finestre dei plugin da `Plugins`/`Hosted` a `EditorHost` (os-app), ma i percorsi x11 usavano ancora i campi vecchi:

- `browser.rs`: i frame per il drop di campioni vengono da `self.editors.editors` (non da `Plugins::hosted[].window`).
- `playlist.rs` (`Cmd::WaveSize`): `app.editors.close_window` + `app.editors.open_window` al posto di `Plugins::close_window`/`open_window`.
- `app.rs` (dialogo aperto): `self.editors.frames.unpinned()` restituisce il `Box<dyn Any>` atteso dal campo, invece del tipo x11 diretto.
- `app.rs` (dropout log): conversione esplicita `transport::Drops { feeder, plugin, device, heard }` -> `dropout_log::Drops { empty, plugin, device }`; il `From` implementato e' solo per `os_session::transport::Drops`.
- `layout.rs`: snapshot/restore delle finestre editor leggono e scrivono `EditorHost::editors` (posizione, detached, open) e passano l'host a `project_snapshot`; aggiornati i cinque chiamanti e il test Windows-only di `transport.rs`.

Verifiche locali (macOS, stessi env di §24.2):

- `cargo check -p os-session --all-targets`: **ok**.
- `cargo check -p os-app --all-targets`: **ok** (57 s, solo warning pre-esistenti).
- I file Windows-only `device/asio.rs` e `device/wasapi.rs` sono stati parse-checkati con `rustfmt --edition 2021` (ok); la compilazione vera per Linux/Windows resta affidata alla CI, perche' su macOS quei rami `cfg` non vengono compilati.
- La `cfg` su `state_fail` in `Plugins::state` e' inclusa in questo commit (fix del job macOS).

### 24.8 Secondo giro CI (commit `9f5639c2`, run `45912022368`)

- **Linux: `Build` verde**, quindi i fix os-app Linux compilano. **macOS: `Build` e `Build the session alone` verdi**, quindi la `cfg` su `state_fail` e' corretta.
- Tutti e tre i job falliscono in `Build the session with features off`: `Plugins::prepare_preset` usava `os_vst3::ara::State::is(data)` anche senza la feature `vst3`. La clausola ora e' dietro `#[cfg(feature = "vst3")]` (con `false` quando la feature e' spenta). La matrice locale `cargo check --locked -p os-session --no-default-features` piu' ognuna delle dieci feature di `ci.yml` (vst3, vst2, export-flac, export-mp3, export-ogg, midi-io, device, fl-native, rpc) e' verde.
- **Windows: `Build`**: `os-session` (ASIO/WASAPI) compila; falliva `os-app` con dieci errori della stessa migrazione delle finestre: `Plugins::update_chop_workspace`, `window_of`, `move_docked`, `close_racks`, `wave_size`, `move_into_workspace`, `Hosted::rack_editor`, `plugin_window::new_frame`, `first_position` e `LiftInput`. Correzioni:
  - `update_chop_workspace` e `wave_size` ora vivono in `EditorHost` (il primo conserva `chop_workspace` e rifitta la finestra di ML Chop via `chop_placement`; il secondo imposta la preferenza e riapre la finestra come fa il ramo Linux, cosi' `open_window` riapplica `wave_zoom`).
  - `window_of`, `move_docked`, `close_racks`, `move_into_workspace` e `rack_editor` usano `EditorHost`/`Editor`; aggiornati `visualizer_video.rs`, `arrange.rs`, `detached.rs`, `win_clamp.rs`, `app.rs` (`clip_docked`).
  - `plugin_window::new_frame` non e' piu' `cfg(not(windows))` (il corpo aveva gia' il ramo win32); `first_position` e' quello gated, perche' su Windows arriva da `win32`. `LiftInput` e' definito su tutte le piattaforme (il trait `Frames` lo nomina sempre).
- Verifica locale: `cargo check -p os-app --all-targets` verde; `os-session --no-default-features` verde; i file Windows-only passano `rustfmt --edition 2024` (solo parse, non tipi). La compilazione Windows vera sara' la run sul commit successivo.
- Release ancora non lanciata: serve una CI verde sul commit da pubblicare.

### 24.9 Terzo giro CI (commit `d81f584e`, run `45916885025`)

- **Linux: verde** e **macOS: verde**, tutte le step (`Build`, sessione sola, feature spente). Il fix `os_vst3` e' quello giusto.
- **Windows: `Build`** arriva a un solo errore, un `E0502` in `align_channel_editors`: `let s = &mut self.state` convive con `self.plugins.scale`, che passa per `Deref<Target = Session>` e quindi prende in prestito tutto `*self`. Ora legge `s.plugins.scale` (stesso prestito di `s`); `self.canvas_off` resta un campo diretto di `App`. Gli altri dieci errori della migrazione sono spariti.
- Prossimo passo: commit di questa riga, push, nuovo dispatch; il job Windows e' l'unico ancora da confermare.

### 24.10 Quarto giro CI (commit `5b445312`, run `45933477330`)

- **Linux e macOS: verdi** (Build, sessione sola, feature spente, bundle, test non bloccanti).
- **Windows**: la `lib` compila (il fix `s.plugins.scale` era giusto); falliscono gli **otto errori dei test Windows-only**, rimasti sulle API pre-migrazione:
  - `visualizer_video.rs` e `legacy_vst.rs`: `app.state.project = ...` assegnava un `Project` a `Locked`; ora `*app.state.project = ...`.
  - `visualizer_video.rs`: `plugins.added(key)` vuole lo `SettingsStore`; ora `app.state.session.plugins.added(&app.state.session.settings, key)`.
  - `strip_editor.rs`: il test di creazione rack usa un `EditorHost::default()` e `editors.show_rack(&mut plugins, ...)`, poi legge `editors.editors[&key].rack_editor/rack_error`.
  - `transport.rs`: `a.plugins.idle(...)` e' diventato `a.editors.idle(&mut a.state.session.plugins, ...)`.
- Verifica locale: `cargo check -p os-app --all-targets` verde su macOS; i quattro file passano `rustfmt --edition 2024`. I test Windows-only non sono compilabili localmente (serve MSVC), quindi la conferma viene dalla run successiva.





## 25. Test rossi nascosti dalla CI verde (2026-11-27, in corso)

La CI su `7e4f7ae1` era verde solo perché lo step `Test` è `continue-on-error`: Linux 75, Windows 72 test os-app rossi; macOS si fermava a `ml-mix-core` (il resto non girava). Locale macOS (`/Users/ade/aqbuild/t.sh <pacchetto> <log> [filtri]`, stessi env di §24.2): os-app **74 → ~17**, os-theme verde, ml-mix-core verde.

**Direzione (owner, 2026-11-27):** AQUILA segue Lemon Studio, ML Studio non è più riferimento, ma **tutto ciò che è già arrivato da ML resta**. Nei conflitti si tengono entrambi, oppure il comportamento ML adattando il test Lemon; i test Lemon di UI sostituita vanno `#[ignore = "..."]` come in §5.3.

**Commit locali, NON pushati:** `bfe1fa0d`, `2118330a`, `d6c8d30b` + modifiche non committate (trad meter in os-theme, Add sample folder, test splash/recording_pickup/plugin_downloads/browser, SoundFont nei gruppi factory, messaggio di helper_install). Copia della prima diff: scratchpad `wip-ci-fixes.patch`.

**Causa ricorrente:** campi doppi tra `State` (app, verità di AQUILA) e `Session` (Lemon): `song_mode`/`playlist.pos`/`piano.marker`, `tempo`, `clip_colors`, `link_moves`, `params` vs `session.params`, `params.members` vs `session.plugin_members`, id non assegnati prima di `control::edit`. Copie duplicate di moduli (`ml_patch::core` → ora riesporta `os_session::ml_patch`). Se un test fallisce in modo strano, cercare prima un doppione.

**Ancora rossi (os-app, macOS):** `ghost_note_double_right_click_and_pattern_open`, `playlist_floating_frame_*`, `playlist_scroll_at_clipped_window_edges`, `playlist_scroll_moves_pixels`, `scroll_repaint_matches_full`, `selection_fixes_pattern_clips_follow_note_edits_and_history`, `studio_rack_keeps_pitch_knobs_and_inline_graph_targets`, `tempo_tapper_flash_repaints_the_pad`, `traditional_mixer_layouts_controls_and_persistence`, `transport_clock_digits_share_a_bottom_line` (rosso anche su ML pulito), `window_move_resize_min_max`, `help::guide_menu_browse_search_read_picture_and_visibility`, `mixer::undoing_a_track_delete_puts_back_its_slots_colours_and_icons` (Lemon; le mappe di `State.mixer` non tornano con l'undo di progetto), `win_clamp` ×2, `ai_control` (ambiente macOS, §24.3), `melli_archive` (instabile in parallelo). Da verificare dopo l'ultima modifica: splash, recording_pickup, factory_folders, catalog_browser_search.
**Fuori da os-app:** os-session 4 (`every_lemon_effect_is_built_in`, `wave_factory_organization` "ML Amp" senza categoria, `audit_updated_zip_equal_lengths`, `the_stopped_playback_keeps_the_stream`), `watermark::every_source_file_has_the_header` (forse solo perché la copia di build non ha `.git`).

**Prossimi passi:** ricompilare e rieseguire os-app, committare, poi `cargo test --workspace --no-fail-fast` locale, push su `ade/michael-lane`, dispatch della CI (`gh workflow run ci.yml --ref michael-lane -R ajdohaxhia/AQUILA`) e controllare i log dello step Test per Linux/Windows (`gh run view <id> --log | grep "test result"`). Solo dopo valutare di togliere `continue-on-error`.

### 25.1 Aggiornamento (stessa sera)

- os-app macOS: **17 rossi** (elenco sopra meno splash, recording_pickup, factory_folders, catalog_browser_search).
- os-session: **1 rosso**, `transport::the_stopped_playback_keeps_the_stream_while_its_tails_ring` (test Lemon): allo Stop la coda di release ha un frame muto al passaggio playback → motore live (frame ~930 di 19200). È un vero buco audio nel transport fuso, non un'attesa stale; il messaggio del test ora dice dove.
- Corretti: ML Patch nell'elenco dei built-in, categorie factory per gli effetti ML senza feature (`plugin_db::category`), `.lmn` tra i `PROJECT_EXTS` della sessione (uno zip con un progetto AQUILA non si apriva).
- CI dispatchata su `810c7742` (run `46063618396`), prima di questi ultimi fix: leggere lo step Test di Linux/Windows.

### 25.2 Merge Lemon upstream/main 0aa419ce (2026-11-28) — committato in locale, NON verificato, NON pushato

Conflitti risolti: `live.rs` (versione Lemon con `any` + `clear()` AQUILA), `zipped.rs`, `transport.rs` (GUI transport AQUILA + `Sent`/revisioni Lemon), `capture.rs` (`FrameRing` AQUILA + mutex `consumer` Lemon), `os-clap` (buffer preallocati Lemon + MPE/slide AQUILA, `fixed_str`), `engine.rs` (lato AQUILA; `multicore.rs` resta cancellato), `Cargo.lock` (nostro), `os-app/Cargo.toml` invariato (AQUILA usa ancora vst e encoder). Prossimo passo: `cargo check --workspace` con gli env di §24.2 (serve `ML_PLASMA_BUNDLE_MACOS_AARCH64`), aggiornare `Cargo.lock`, test, poi push e CI.
