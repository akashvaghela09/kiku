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

## Chunk 6 — Overlay · `TODO`
Transparent, always-on-top, click-through, non-activating window · idle → listening →
processing → done → error · amplitude-driven waveform · multi-monitor placement ·
reduced-motion support.

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

## Chunk 10 — Settings + onboarding · `TODO`
First-run: welcome → model choice → download → permissions → first dictation.
Settings: hotkeys, microphone, model, sounds, history, update check, about.

## Chunk 11 — Update check · `TODO`
Once-daily cached GitHub Releases call · semver compare · banner with link · never
downloads · switchable off · silent when offline.

## Chunk 12 — Packaging · `TODO`
AppImage + deb · MSI/NSIS · dmg (aarch64, ad-hoc signed) · GitHub Actions release ·
README with Gatekeeper/SmartScreen instructions · NVIDIA model attribution.

---

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
