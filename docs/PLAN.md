# Kiku — Build Plan

Built in sequential chunks. Each chunk ends in something runnable and verifiable.
Status legend: `TODO` · `WIP` · `DONE` · `BLOCKED`

---

## Chunk 0 — Engine spike (gate) · `DONE`

A throwaway headless Rust CLI proving the engine before any app exists.

- [ ] `spike/asr-probe` Cargo project: `sherpa-rs`, `cpal`, `hound`, `anyhow`, `clap`
- [ ] Download + pin the 0.6B v2 int8 model, verify checksums
- [ ] `probe file <wav>` — transcribe a WAV, print text + wall-clock + RTF
- [ ] `probe mic -s N` — record N seconds at 16 kHz mono, transcribe, print
- [ ] `probe bench` — RTF across clip lengths (2s / 5s / 15s / 60s)
- [ ] Verify the 110M CTC model loads through the same code path
- [ ] **Owner gate: transcribe Akash's own voice and judge the accuracy**

**Answers:** do the ONNX exports work with the Rust bindings; is CPU fast enough; is
accuracy acceptable on the owner's voice. A failure here reopens the Whisper decision.

**Findings so far:** `sherpa-rs` v0.6.8, actively maintained. Official exports exist:
- default `csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8` @ `1ab93235…` (631 MB, transducer)
- small `csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000` @ `3af92f15…` (437 MB, CTC)

The two models are **different architectures** (transducer vs CTC), so the engine layer
must abstract over both. The official int8 110M repos are empty placeholders; the small
model is therefore the 437 MB non-int8 CTC build, not the ~200 MB originally assumed.

---

## Chunk 1 — Skeleton · `DONE`
Tauri 2 + React/TS/Tailwind/Lucide scaffold · domain-module layout · `tauri-specta`
typed IPC · icon pipeline from the brand master · strict TS · clippy/rustfmt/ESLint ·
CI building all three targets.

## Chunk 2 — Audio capture · `DONE`
`cpal` 0.18 device enumeration keyed on `DeviceId` (survives replugging better than a
name) · opens the microphone at 16 kHz natively where the device allows it, skipping
our resampler entirely · rubato FFT resampling otherwise, with tests proving a 15 kHz
tone is attenuated rather than aliased down into the speech band · RMS/peak metering
with clipping detection, computed in the audio callback to drive the overlay waveform ·
10-minute recording ceiling · capture runs on its own thread because `cpal::Stream` is
not `Send`, and device-open failures surface at `start()` rather than at `stop()`.

## Chunk 3 — Engine layer · `DONE`
`Engine` trait with one implementation (Parakeet via sherpa-onnx) · a single warm
engine for the process lifetime, since loading costs ~4 s against a sub-second
dictation budget · four-state status so the UI can say "still getting ready" rather
than appearing to ignore a hotkey · decode capped at 4 threads, where chunk 0's
measurements showed returns flatten, leaving cores for the waveform · integration test
runs the real model and asserts both the text and the real-time factor, skipping
itself when no model is installed.

## Chunk 4 — Model management · `DONE`
Registry pins a Hugging Face commit, never a branch, and every file carries the
SHA-256 it must hash to (verified against Hugging Face's own LFS object ids) ·
resumable download into `.part` files, renamed into place only after the hash matches,
so a complete-looking model is always a correct one · a server that ignores a Range
request is detected rather than corrupting the resume · cheap startup check on
presence and size, expensive hash check only after download or on explicit repair ·
byte counts cross to the frontend as `f64`, which is exactly what a JavaScript number
is · network integration test exercises the real endpoint using only the 9 KB tokens
file.

## Chunk 5 — Global hotkeys · `DONE`
`Alt+Space` hold-to-talk and `Ctrl+Alt+Space` toggle · the hold/toggle semantics and
key-repeat suppression live in a pure `Interpreter` that is tested without an OS ·
rebinding rolls back to the previous pair if registration fails, so a conflict can
never leave the user with no working hotkey · a hotkey already owned by another
application is a warning at startup, not a failure to launch · also lands the
`Dictation` session that hotkeys drive, with audio levels throttled from the audio
callback rate to ~14 Hz before they reach the overlay.

## Chunk 6 — Overlay · `DONE`
Transparent, always-on-top, click-through capsule, verified running on this machine ·
placed on the monitor under the cursor, anchored to the **work area** so it clears the
taskbar, recomputed on every show · bars radiate from the centre rather than scrolling
left to right, because a scrolling waveform reads as a recording timeline and Kiku
stores no audio · the waveform loop writes `transform` imperatively inside one rAF
callback and never calls `setState` · RMS mapped through dB, fast-attack/slow-release
smoothing, and a breathing pulse at silence so it never reads as crashed ·
reduced-motion drops to 10 Hz rather than stopping, because the waveform answers "am I
being heard?" and that is information · Escape cancels, grabbed only while a session is
running.

## Chunk 7 — Output · `DONE`
Clipboard write is unconditional; the paste is synthesised on top of it, so a refused
paste still leaves the text somewhere reachable · every modifier is released first,
because hold-to-talk fires on key release and a user commonly lifts the space bar
before Alt — without this, Ctrl+V would really be Ctrl+Alt+V · a settle delay covers
X11's clipboard ownership handshake · the paste modifier is released even when the
keystroke fails, so a failure cannot leave Ctrl stuck down across the desktop ·
whitespace is normalised (newlines included, which would otherwise submit a form),
punctuation and capitalisation are left to the model.

## Chunk 8 — Audio feedback · `DONE`
Three cues synthesised in code — no audio files, so no licence to carry and no bytes
in the bundle · a rising blip to start, falling to finish, low double to discard ·
phase-accumulated rather than evaluated per sample, and faded at both ends, because
either mistake produces an audible click on every playback · tests assert the waveform
is continuous and starts and ends at silence, which is what a click actually is · each
cue opens an output stream and closes it rather than holding the audio device open for
the whole session · a cue that cannot play is logged, never surfaced.

## Chunk 9 — History · `DONE`
SQLite with a migration runner present from the first release, which is the only way
"survives updates" can actually hold · FTS5 search over external content, so a deleted
transcript cannot survive in the search index · user input is sanitised into FTS5
syntax, since a search box containing a quote or `AND` would otherwise be a syntax
error · pause recording, delete one, delete all, and an age-based purge · a history
that fails to open is logged and dictation continues, because refusing to launch over
a history problem is the worse failure. UI lands in chunk 10.

## Chunk 10 — Settings + onboarding · `DONE`
Fourteen interface primitives, no more — `Row` and `Panel` carry most of the app, and
Settings contributes no components of its own · Settings is a sticky section list
beside one continuous scroll rather than tabs, so nothing that might be blocking a
user is hidden behind a click · rebinding captures a real key press instead of asking
someone to type `Ctrl+Shift+Space` into a box · onboarding is three steps: get the
model, grant the microphone, try it once · all three surfaces verified running.

## Chunk 11 — Update check · `DONE`
One GitHub Releases call, cached for a day, comparing semver and returning a URL ·
never downloads, installs or executes anything · drafts and prereleases ignored · a
non-semver tag reports Unknown rather than guessing · offline is `Unknown`, not an
error, because a failed update check is not worth interrupting anyone about · the
version comparison is separated from the fetch so it is tested without a network.
Banner UI lands in chunk 10.

## Chunk 12 — Packaging · `DONE` (Linux verified; Windows and macOS untested)
The sherpa-onnx shared libraries ship as bundle resources, listed per platform, with
rpath entries covering both `cargo run` and an installed layout · **verified on
Linux**: the deb places the binary at `/usr/bin/kiku` and the libraries at
`/usr/lib/Kiku/lib`, and both the deb and the AppImage start, resolve every library
and load the speech model · CI runs lint, typecheck, clippy and tests on every push ·
release is tag-driven and drafts a GitHub release for all three platforms · README
covers the Gatekeeper and SmartScreen prompts that an unsigned build produces.

**Untested:** the Windows and macOS bundles. The configuration is written and the
matrix builds them, but neither has been run. The likely trouble spots are the macOS
`@executable_path/../Resources/lib` rpath and whether the DLL names match what the
build script emits.

**Known cosmetic issue:** linuxdeploy copies the libraries a second time into
`/usr/lib`, so the AppImage carries about 21 MB it does not need.

---

## Follow-ups

- **Hotkey presets.** `F9` / `F10` are offered in Settings and are the easiest keys to
  *hold*, but Mac laptops map function keys to media unless standard function keys are
  enabled. The shipped default stays `Ctrl+Shift+Space` / `Ctrl+Alt+Space`, which works
  identically on all three platforms with no setup.
- **`Alt+Space` is confirmed taken** on the development machine (`activate-window-menu`
  in Cinnamon) and is the window menu on Windows too, which is why it is not the
  default despite reading better.
- **Synthetic key events do not trigger the X11 grab**, so the hotkey path could not be
  exercised with `xdotool`. Everything downstream of it is verified; the key press
  itself needs a physical test.

## Cross-cutting standards

**Rust** — domain modules not layer modules · `clippy -D warnings` in CI · `thiserror`
for typed errors · no `unwrap()` outside tests · one abstraction (`Engine`), no
speculative others.

**TypeScript** — `strict: true` · `any` banned by lint · feature-folder slices ·
`invoke()` called only from `src/lib/ipc/`, never from components.

**Styling** — design tokens as CSS custom properties; no literal hex in components, so
light/dark and any rebrand are a one-file change.

**IPC** — `tauri-specta` generates TS types from the Rust command signatures, so a
renamed field is a compile error rather than a runtime surprise.
