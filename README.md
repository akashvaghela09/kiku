<div align="center">

<img src="assets/brand/kiku-icon-1024.png" width="112" alt="Kiku">

# Kiku

**Offline dictation for your desktop.**
Hold a key, speak, and the text appears where you are already typing.

[![Download](https://img.shields.io/badge/Download-latest-22A7CC?style=for-the-badge)](https://github.com/akashvaghela09/kiku/releases/latest)
[![Platforms](https://img.shields.io/badge/Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-1C8FAF?style=for-the-badge)](https://github.com/akashvaghela09/kiku/releases/latest)
[![License](https://img.shields.io/badge/License-MIT-2A94B2?style=for-the-badge)](LICENSE)

### [Download the latest release](https://github.com/akashvaghela09/kiku/releases/latest)

</div>

---

## What it does

You are writing an email. Instead of typing the next paragraph, you hold a key, say it
out loud, and let go. The words appear at your cursor.

I built Kiku because every dictation tool I tried wanted an account, an internet
connection, or both, for a job that a modern laptop can do on its own. Speech
recognition here runs entirely on your machine. There is nothing to sign into, no audio
is ever written to disk, and nothing you say leaves the computer. Kiku touches the
network exactly twice: once to download the speech model, and, if you leave the setting
on, once a day to ask whether a newer version exists.

## Shortcuts

One key, two ways to use it.

| Action | Windows | Linux | macOS |
| :-- | :-- | :-- | :-- |
| **Hold to talk** | Hold Right Ctrl | Hold Right Ctrl | Hold Right Option |
| **Hands free** | Double-tap Right Ctrl | Double-tap Right Ctrl | Double-tap Right Option |
| **Stop hands free** | Tap Right Ctrl | Tap Right Ctrl | Tap Right Option |
| **Cancel** | Esc | Esc | Esc |

Only the key differs, and only on macOS: Mac keyboards have no right Control key, so
Right Option takes its place. The gestures are identical everywhere.

Holding records for as long as you hold, and the text is pasted when you let go.
Double-tapping keeps listening until you tap once more, so you can dictate with both
hands free. Escape throws a recording away without transcribing it.

Both modes start on the one key. Settings binds them separately, so hands free can have
a key of its own - any right-hand modifier, since those are the keys Kiku can watch
being tapped twice. The defaults live in
[`src-tauri/src/hotkeys/defaults.rs`](src-tauri/src/hotkeys/defaults.rs), the only file
to edit to change what a fresh install gets.

A bare modifier is deliberate: one key is far easier to *hold* than a chord, and no
operating system claims a right-hand modifier on its own. No platform will register a
lone modifier as a shortcut, so Kiku watches the key rather than capturing it. It
therefore keeps working as an ordinary modifier everywhere else, and **pressing any
other key while holding cancels the recording** - which is what stops Right Ctrl + C
from copying and dictating at the same time.

Closing the window does not stop Kiku: the dictation key goes on working and the tray
icon stays. Quit from there when you want it to stop.

## Installing

Kiku is not code signed, so Windows and macOS each warn you once on first launch, and
macOS needs two permissions granted by hand. Linux needs neither.

<details>
<summary><b>Linux</b></summary>

- Install a package, or run the AppImage:

  ```sh
  sudo apt install ./Kiku_*_amd64.deb     # Debian, Ubuntu
  sudo dnf install ./Kiku-*.x86_64.rpm    # Fedora, RHEL

  chmod +x Kiku_*_amd64.AppImage && ./Kiku_*_amd64.AppImage
  ```

- Launch Kiku. Hold **Right Ctrl**, say a few words, and let go.

No permissions to grant, and nothing to approve. Run the commands from wherever the
file was downloaded; the wildcards keep them working after a new version.

X11 only for now. Wayland needs portal based shortcuts and `uinput` for pasting, and I
have not built that yet.

</details>

<details>
<summary><b>Windows</b></summary>

- Run `Kiku_*_x64-setup.exe`.
- SmartScreen says "Windows protected your PC". Choose **More info**, then **Run
  anyway**.
- Launch Kiku. Hold **Right Ctrl**, say a few words, and let go - Windows asks for the
  microphone the first time.

Windows has not been tested yet; these steps may need more detail once it has been.

</details>

<details>
<summary><b>macOS (Apple Silicon)</b></summary>

One approval and two permissions, in this order. Each is asked for once.

- Open `Kiku_*_aarch64.dmg` and drag Kiku to Applications.
- Launch it. macOS refuses: *"Apple could not verify Kiku is free of malware."*
- Open **System Settings > Privacy & Security**, scroll to the Security section, and
  choose **Open Anyway** beside Kiku. Launch it again and choose **Open**.
- Tap **Right Option twice** to start listening, say a few words, and tap again.
  macOS asks for the **microphone**; allow it.
- Dictate once more. Nothing is pasted yet, because Kiku still needs
  **Accessibility** to type into other applications. Open **System Settings > Privacy
  & Security > Accessibility**, and switch Kiku on.
- Dictate a third time. The text now lands where your cursor is.

Without Accessibility, dictation still works and the transcript goes to the clipboard -
you just paste it yourself.

**After updating**, Kiku may stop pasting and the dictation key may stop responding.
Kiku is not notarized, so each build is a new application as far as macOS is concerned,
and the existing Accessibility entry no longer matches it. Remove Kiku from **Privacy &
Security > Accessibility** with the **-** button, then add it again.

</details>

## Speech models

Two, both English only. Settings lets you download either one, switch between them, and
delete one to get the disk space back.

| Model | Size | Notes |
| :-- | --: | :-- |
| **Standard** | 631 MB | The default. Around fifteen times faster than real time on four cores, and six times faster on one. |
| **Compact** | 455 MB | Lighter on the processor and on disk, slightly less accurate. |

Neither needs a GPU. I measured both engines before writing any of the application, and
chose NVIDIA Parakeet over Whisper because it is more accurate on English, its cost
scales with how long you actually spoke rather than with a fixed window, and it stays
silent when you do instead of inventing words. The numbers behind that decision are in
[`docs/CHUNK-0-RESULTS.md`](docs/CHUNK-0-RESULTS.md).

Switching models takes a few seconds. Almost all of it is the ONNX runtime building its
session, which no amount of threading improves, so Kiku tells you what it is doing
rather than pretending to be quick.

## What else is in there

- **History.** Every transcript, searchable, stored as text on your machine. Long ones
  clamp to two lines with a Read more. Audio is never saved.
- **A listening indicator.** A small capsule floats above whatever you are working in.
  Seven marks track your voice while you talk, then collapse into a pulse while the
  text is being transcribed and pasted. They respond to speech rather than to room
  noise, because Kiku learns your microphone's noise floor instead of assuming one.
- **Sound cues** when listening starts and stops, switchable off.
- **Privacy controls.** Turn off Record history to stop saving transcripts, delete one
  or all of them, or have anything older than 7, 30 or 90 days removed automatically.

## Building it yourself

You will need [Rust](https://rustup.rs) and [Node 22 or newer](https://nodejs.org).
Then, on any platform:

```sh
npm install
npm run app          # run in development
npm run app:build    # bundle installers for this platform
```

Installers land in `src-tauri/target/release/bundle/`. Each platform needs its own
toolchain first.

<details>
<summary><b>Linux</b></summary>

```sh
sudo apt install libwebkit2gtk-4.1-dev libasound2-dev build-essential curl file \
                 libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Builds an `.AppImage` and a `.deb`. X11 only.

</details>

<details>
<summary><b>macOS</b></summary>

```sh
xcode-select --install
```

Builds a `.dmg`, natively for whichever architecture you are on.

</details>

<details>
<summary><b>Windows</b></summary>

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
with the **Desktop development with C++** workload. WebView2 is already present on
Windows 11 and ships with the installer on Windows 10.

Builds an NSIS `-setup.exe`.

</details>

Useful commands:

```sh
npm run check                    # eslint and tsc

cd src-tauri
cargo test --lib                 # unit tests, no model or network needed
cargo test export_bindings       # regenerate src/lib/ipc/bindings.ts from Rust
KIKU_TEST_NETWORK=1 cargo test   # includes the download test
cargo clippy --all-targets -- -D warnings
```

`src/lib/ipc/bindings.ts` is generated from the Rust command signatures and committed,
so a fresh clone typechecks without a Rust build. CI regenerates it and fails if it
differs, which is what stops a stale copy from quietly compiling.

## Continuous integration

Two workflows, split by what they are for.

**CI** runs on every push and pull request. On Linux it checks formatting, lint, types,
clippy and the test suite, and rebuilds `bindings.ts` to prove the committed copy is
current. It then compiles on Windows and macOS as well, without linking or running
anything, because a module can be perfectly valid on Linux and dead code on the other
two, and that is not something a Linux machine can tell you.

**Release** runs only on a `v*` tag or a manual dispatch. The steps are in
[`docs/RELEASING.md`](docs/RELEASING.md). It builds installers on all
three platforms and attaches them to a **draft** release, so publishing stays a
decision rather than a side effect of pushing. It does not cache `target/`: a restored
build directory lets cargo treat the native dependency's build script as fresh while
the libraries it extracted are gone, which fails the link in a way that takes a long
time to recognise. A release is built from clean on purpose.

## Under the hood

| | |
| :-- | :-- |
| Recognition | [NVIDIA Parakeet](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2) via [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx), CPU only |
| Application | Tauri 2, Rust, React, TypeScript, Tailwind |
| Storage | SQLite in your platform's application data directory |
| Audio | 16 kHz mono capture, anti aliased resampling, adaptive noise floor |

The reasoning behind the bigger decisions lives in [`docs/`](docs/): `SCOPE.md` for what
Kiku is and is not, `DESIGN.md` for the interface, `PLAN.md` for how it was built,
`CHUNK-0-RESULTS.md` for the measurements that chose the engine, and
[`RELEASING.md`](docs/RELEASING.md) for how a release is cut.

## Attribution

Speech recognition uses NVIDIA's Parakeet model, licensed
[CC-BY-4.0](https://creativecommons.org/licenses/by/4.0/). Model conversion to ONNX by
the [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) project.

## Licence

MIT. See [LICENSE](LICENSE).
