<div align="center">

<img src="assets/brand/kiku-icon-1024.png" width="112" alt="Kiku">

# Kiku

**Offline dictation for your desktop.**
Hold a key, speak, and the text appears where you are already typing.

[![Download](https://img.shields.io/badge/Download-v1.0.7-22A7CC?style=for-the-badge)](https://github.com/akashvaghela09/kiku/releases/latest)
[![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-1C8FAF?style=for-the-badge)](https://github.com/akashvaghela09/kiku/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-2A94B2?style=for-the-badge)](LICENSE)

### [Download the latest release](https://github.com/akashvaghela09/kiku/releases/latest)

</div>

---

## What it does

You are writing an email. Instead of typing the next paragraph, you hold a key, say it
out loud, and let go. The words appear at your cursor.

Everything happens on your own computer. Speech recognition runs locally, there is no
account to create, and nothing you say is ever sent anywhere or written to disk as
audio. The network is used exactly twice: once to download the speech model, and, if
you leave the setting on, once a day to check whether a newer version exists.

## Shortcuts

| Key | What happens |
| :-- | :-- |
| **Right Ctrl** (hold) | Records while held. Let go and the text is pasted. |
| **Right Ctrl** (tap twice) | Hands free. Recording continues until you tap again. |
| **Ctrl + Alt + Space** | Toggle, if you prefer a chord. |
| **Escape** | Cancel without transcribing. |

On macOS this is **Right Option**, because Mac keyboards have no right Control key.

One key is easier to hold than a chord, and no operating system uses Right Ctrl on its
own. Kiku watches the key rather than capturing it, so Right Ctrl keeps working as
Ctrl everywhere else, and **pressing any other key while holding cancels**. That is
what stops Right Ctrl + C from copying and dictating at the same time.

Every shortcut can be changed in Settings, which also offers an F9 / F10 preset and a
Ctrl + Shift + Space chord.

## Installing

Kiku is not code signed. That is deliberate: certificates cost money every year and
buy nothing for a tool you can build yourself from this repository. It does mean each
operating system warns you once.

<details>
<summary><b>Linux</b></summary>

Download the AppImage, make it executable, and run it:

```sh
chmod +x Kiku_1.0.7_amd64.AppImage
./Kiku_1.0.7_amd64.AppImage
```

Or install the Debian package:

```sh
sudo apt install ./Kiku_1.0.7_amd64.deb
```

X11 only for now. Wayland needs portal based shortcuts and `uinput` for pasting, which
is not built yet.

</details>

<details>
<summary><b>Windows</b></summary>

Run the installer. SmartScreen will say "Windows protected your PC". Choose
**More info**, then **Run anyway**.

</details>

<details>
<summary><b>macOS (Apple Silicon)</b></summary>

Open the disk image and drag Kiku to Applications. The first launch is refused: open
**System Settings > Privacy & Security**, scroll down, and choose **Open Anyway**.

macOS will then ask for two permissions, both under Privacy & Security:

- **Microphone**, so Kiku can hear you.
- **Accessibility**, so Kiku can paste into other applications. Without it your
  transcript still reaches the clipboard and you paste it yourself.

</details>

## Speech models

Two, both English. Settings lets you download either one, switch between them, and
delete one to get the disk space back.

| Model | Size | Notes |
| :-- | --: | :-- |
| **Standard** | 631 MB | The default. Around fifteen times faster than real time on four cores, and six times faster on one. |
| **Compact** | 455 MB | Lighter on the processor and on disk, slightly less accurate. |

There is **no GPU requirement**. That is the reason Kiku uses NVIDIA Parakeet rather
than Whisper: it is more accurate on English, its cost scales with how long you
actually spoke, and it stays silent when you do, instead of inventing words.

## What else is in there

- **History.** Every transcript, searchable, stored as text on your machine. Long ones
  collapse to three lines with a Read more. Audio is never saved.
- **A listening indicator.** A small capsule floats above whatever you are working in.
  Its waveform responds to your voice, not to room noise: Kiku learns your
  microphone's noise floor rather than assuming one.
- **Sound cues** when listening starts and stops, switchable off.
- **Privacy controls.** Pause history, delete one entry or all of them, or have
  anything older than a chosen age removed automatically.

## Building it yourself

```sh
npm install
npm run app          # run in development
npm run app:build    # bundle for this platform
```

You will need Rust and Node. On Debian or Ubuntu, also:

```sh
sudo apt install libwebkit2gtk-4.1-dev libasound2-dev build-essential curl file \
                 libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Useful commands:

```sh
cd src-tauri
cargo test                       # unit tests, no model or network needed
cargo test export_bindings       # regenerate src/lib/ipc/bindings.ts from Rust
KIKU_TEST_NETWORK=1 cargo test   # includes the download test
cargo clippy --all-targets -- -D warnings
```

## Continuous integration

Every push to `master` builds installers for all three platforms and uploads them to
the workflow run, downloadable for 30 days, so there is always something to test
without building locally. Pushing a `v*` tag builds the same installers and attaches
them to a draft release, so publishing stays a decision rather than a side effect.

## Under the hood

| | |
| :-- | :-- |
| Recognition | [NVIDIA Parakeet](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) via [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx), CPU only |
| Application | Tauri 2, Rust, React, TypeScript, Tailwind |
| Storage | SQLite in your platform's application data directory |
| Audio | 16 kHz mono capture, anti aliased resampling, adaptive noise floor |

Design notes and the build log are in [`docs/`](docs/): `SCOPE.md` for what Kiku is and
is not, `PLAN.md` for how it was built, and `CHUNK-0-RESULTS.md` for the measurements
that chose the engine.

## Attribution

Speech recognition uses NVIDIA's Parakeet model, licensed
[CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/). Model conversion to ONNX by
the [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) project.

## Licence

MIT. See [LICENSE](LICENSE).
