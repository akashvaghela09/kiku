/**
 * The only module in the app that talks to Rust.
 *
 * Components import the typed helpers exported here and never call `invoke` or
 * `listen` themselves, so the IPC surface stays greppable and mockable in one place.
 *
 * `bindings.ts` is generated from the Rust command signatures — regenerate it with
 * `cargo test export_bindings` inside `src-tauri/`. It is gitignored deliberately: a
 * stale checked-in copy is worse than no copy, because it still compiles.
 */
export { commands, events } from './bindings';
export type {
  AppInfo,
  DictationState,
  Discarded,
  DownloadProgress,
  EngineStatus,
  Entry,
  ErrorPayload,
  Hotkey,
  HotkeyBindings,
  InstallState,
  Level,
  MicrophoneInfo,
  ModelInfo,
  Page,
  Preferences,
  SoundCue,
  Transcript,
  UpdateStatus,
} from './bindings';
