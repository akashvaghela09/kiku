# Chunk 0 - Engine gate results

**Verdict: PASSED.** Parakeet on CPU is comfortably fast enough. The Whisper decision
stays closed, and no GPU code is needed anywhere in Kiku.

Measured on the development machine: Linux Mint (Cinnamon, X11), 16 logical cores,
PipeWire, `sherpa-rs` 0.6.8, `parakeet-tdt-0.6b-v2-int8` (631 MB), 8 inference threads.

## Accuracy

Reference human clip shipped with the model (7.43 s):

> Well, I don't wish to see it any more, observed Phebe, turning away her eyes.
> It is certainly very like the old portrait.

Synthetic clip with known text (8.24 s) - transcribed **exactly**, including the
product name and both sentence boundaries:

> The quick brown fox jumps over the lazy dog. Kiku is an offline dictation tool
> that runs entirely on your own machine.

Punctuation and capitalisation come out of the model directly. No post-processing
needed.

## Speed

| Clip length | Decode | RTF | vs real time |
|---|---|---|---|
| 2 s | 0.17 s | 0.085 | 11.7× |
| 5 s | 0.33 s | 0.065 | 15.3× |
| 15 s | 0.78 s | 0.052 | 19.1× |
| 60 s | 4.08 s | 0.068 | 14.7× |

Model load: **4.2 s**, paid once at startup - so the engine must be loaded eagerly and
kept warm, never loaded per dictation.

### Thread scaling (5 s clip)

| Threads | Decode | vs real time |
|---|---|---|
| 1 | 0.82 s | 6.1× |
| 2 | 0.49 s | 10.2× |
| 4 | 0.31 s | 15.9× |
| 8 | 0.27 s | 18.4× |

Returns flatten after 4 threads. Default to `min(4, cores/2)` and leave headroom for
the UI - chasing the last 15% would make dictation compete with the interface drawing
the waveform.

## Findings that change the plan

1. **The small model is probably unnecessary.** Even single-threaded the 0.6B runs 6×
   faster than real time. A machine too weak for that is too weak for a webview. The
   110M model is therefore deferred, not cut - revisit only if a real device proves it
   necessary. This removes the model picker from the first-run flow entirely.
2. **`sherpa-rs` exposes no NeMo-CTC wrapper** (the `nemo_ctc` config field is always
   zeroed). Supporting the 110M CTC export would mean hand-written `sherpa-rs-sys` FFI.
   Another reason to defer it.
3. **`feature_dim` is 128**, not the more common 80, and `model_type` must be
   `nemo_transducer`. The generic `transducer` default mishandles Parakeet's TDT
   duration outputs.
4. **Linking: the libraries must be bundled, not statically linked.**
   `download-binaries` produces `libsherpa-onnx-c-api.so` / `libsherpa-onnx-cxx-api.so`
   which are *not* on the runtime path - the binary fails to start without
   `LD_LIBRARY_PATH`. Enabling the `static` feature does not fix this: it demands
   `RUSTFLAGS="-C relocation-model=dynamic-no-pic"` (disabling PIE, and with it
   executable ASLR) and then still fails to link, because the prebuilt download ships
   shared objects only:

   ```
   rust-lld: error: undefined symbol: SherpaOnnxCreateOfflineRecognizer
   ```

   Static linking would therefore mean building sherpa-onnx from source in CI on all
   three platforms. **Decision for chunk 12: ship the shared libraries alongside the
   binary with an `$ORIGIN` rpath** (`@loader_path` on macOS), which is the ordinary
   Tauri approach for native dependencies and avoids both the source build and the
   ASLR trade-off.
5. **Microphone input clips.** The default device (44.1 kHz, 2 ch) produced a peak of
   1.015 - above full scale. Chunk 2 must clamp, and should surface a clipping warning
   rather than silently distorting the audio the recogniser sees.
6. **Linear resampling is a placeholder.** 44.1 kHz → 16 kHz by linear interpolation
   aliases. Chunk 2 should use `rubato` and re-measure accuracy afterwards.

## Still outstanding

**The owner's own voice has not been tested.** Everything above used a reference clip
and synthetic speech. Accent and noise robustness is Whisper's one genuine advantage,
and it remains unverified. Run this before trusting the gate completely:

```sh
cd spike/asr-probe
export LD_LIBRARY_PATH="$PWD/target/release:$LD_LIBRARY_PATH"
./target/release/asr-probe mic -s 6
```
