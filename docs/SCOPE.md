# Kiku - Scope

**Kiku** (Japanese 聞く, "to listen") is a local-first, offline English dictation tool
for desktop. Hold a hotkey, speak, and the transcribed text is pasted into whatever
application currently has focus. Speech recognition runs entirely on-device.

## Product principles

1. **Single-purpose.** It dictates. It does not summarise, translate, chat, or sync.
2. **Offline by default.** The network is touched exactly twice: a one-time model
   download, and an optional once-daily version check. Both are user-visible and the
   second is switchable off.
3. **No account, no telemetry, no cloud.** Nothing leaves the machine, ever.
4. **Fast enough to be invisible.** Sub-second from key release to pasted text.

## Decisions

| Area | Decision |
|---|---|
| Stack | Tauri 2, Rust, React + TypeScript, Tailwind CSS, Lucide icons |
| Engine | sherpa-onnx via `sherpa-rs`, **CPU only** - no GPU code anywhere |
| Default model | `parakeet-tdt-0.6b-v2` int8 (transducer), ~631 MB |
| Small model | `parakeet-tdt_ctc-110m` (CTC), ~437 MB - opt-in for older hardware |
| Model delivery | Downloaded on first run, pinned revision, checksum-verified. Not bundled |
| Hotkeys | `Alt+Space` hold-to-talk, `Ctrl+Alt+Space` toggle. Both rebindable |
| Feedback | Floating always-on-top overlay + synthesised audio cues (no sound assets) |
| Output | Clipboard, then synthesised paste into whatever has focus at paste time |
| History | SQLite, **text only**, app-data dir, schema-versioned from the first release |
| Updates | No auto-update. Once-daily GitHub Releases check → banner only, switchable off |
| Platforms | Linux x64 (X11), Windows x64, macOS **aarch64** |
| Distribution | Unsigned, GitHub Releases. Ad-hoc signed on macOS to stabilise TCC grants |
| Identity | `io.github.akashvaghela09.kiku` · display name `Kiku` · binary `kiku` |

## Why Parakeet and not Whisper

- Better English accuracy (~6.0% vs ~7.4% avg WER) at ~40% of the parameters.
- Cost scales with utterance length. Whisper always pays for a padded 30-second
  encoder window, which dominates the 2-15 second clips dictation actually produces.
- Transducer models emit nothing on silence. Whisper hallucinates training artefacts
  ("Thank you for watching"), which is a real failure mode for push-to-talk.
- Fast enough on CPU that **the entire GPU acceleration effort is deleted** - no
  Metal, no Vulkan, no CUDA, no per-platform build matrix.

## Explicitly out of scope

GPU acceleration · Whisper · streaming partial transcription · languages other than
English · Wayland · Intel Macs · code signing and notarisation · app stores ·
custom vocabulary · auto-update · storing audio.

## Accepted trade-offs

- **English only, permanently**, without adding a second engine.
- **Accent and noise robustness is unvalidated** - Whisper's one genuine advantage.
  Chunk 0 exists to test this on the owner's actual voice before anything is built.
- **CC-BY-4.0 model licence** requires NVIDIA attribution in the About screen.
- **Unsigned binaries** mean a one-time Gatekeeper/SmartScreen bypass for users, and
  macOS Accessibility grants may need re-approving after rebuilds.
