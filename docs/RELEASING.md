# Releasing Kiku

A release is cut from a tag. Nothing publishes on its own: the tag produces a **draft**
release, and turning that into a public one is a button you press after you have
installed the thing and seen it run.

The short version:

```
green CI on master  ->  bump the version  ->  green CI again  ->  tag  ->  draft  ->  test  ->  publish
```

## 1. Master must be green

Open the Actions tab and confirm the latest **CI** run on `master` passed. That is the
go signal, and it is the whole reason CI runs on every push: a tag is only safe to cut
because the commit it points at was already proven.

CI has three jobs and all three must be green:

| Job | Runs | Catches |
| :-- | :-- | :-- |
| `check` | Linux | formatting, lint, types, clippy, 159 tests, stale `bindings.ts` |
| `cross-check (macos-14)` | macOS | code that compiles on Linux and not on macOS |
| `cross-check (windows-2025)` | Windows | the same, for Windows |

`check` is the Linux job. It is not called "ubuntu" because it is not a matrix job,
which is the only reason the other two advertise their platform in brackets.

If CI is red, stop. A tag on a red commit produces a broken release or no release at
all, and you will not find out for twenty-five minutes.

Documentation-only pushes skip CI deliberately, so a README commit shows no run at
all. That is not a failure; a Markdown change cannot break a build.

## 2. Bump the version

The version lives in four places and every one of them has to agree. Tauri reads
`tauri.conf.json`, cargo reads `Cargo.toml`, and the lockfile has to match `Cargo.toml`
or the build fails. The README is deliberately not one of them: its install commands
use wildcards, so it never needs touching for a release.

```sh
OLD=1.1.4
NEW=1.1.5

sed -i "s/\"version\": \"$OLD\"/\"version\": \"$NEW\"/" package.json src-tauri/tauri.conf.json
sed -i "3s/^version = \"$OLD\"/version = \"$NEW\"/" src-tauri/Cargo.toml
sed -i "/^name = \"kiku\"$/{n;s/^version = \"$OLD\"/version = \"$NEW\"/}" src-tauri/Cargo.lock
```

Check it took:

```sh
grep -rn "$NEW" package.json src-tauri/tauri.conf.json
sed -n '3p' src-tauri/Cargo.toml
grep -A1 '^name = "kiku"$' src-tauri/Cargo.lock
```

On macOS use `sed -i ''` rather than `sed -i`.

## 3. Run the checks locally first

Faster than waiting for CI to tell you the same thing:

```sh
npm run check                             # eslint and tsc
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

`cargo fmt --check` is worth singling out. It runs in CI and in no other local command,
so it is the one that gets forgotten.

## 4. Push, and wait for green again

```sh
git commit -am "Version 1.1.5"
git push origin master
```

Wait for CI. Do not tag ahead of it.

## 5. Optional: prove the build without spending a tag

Actions tab -> **Build** -> **Run workflow**. Same three-platform build, but it uploads
the installers as workflow artifacts instead of creating a release. Worth doing when
the bundling configuration has changed, since a bundling failure is otherwise only
discovered by a tag.

## 6. Tag

```sh
git tag -a v1.1.5 -m "Kiku 1.1.5"
git push origin v1.1.5
```

The tag is what starts **Build**. It takes roughly twenty-five minutes: Linux, macOS
and Windows in parallel, each compiling from clean.

## 7. Check the draft

Build creates a draft release and attaches the installers. It should carry, at minimum:

- `Kiku_<version>_amd64.AppImage` and `Kiku_<version>_amd64.deb`
- `Kiku-<version>-1.x86_64.rpm`
- `Kiku_<version>_aarch64.dmg`
- `Kiku_<version>_x64-setup.exe` and `Kiku_<version>_x64_en-US.msi`

If a platform is missing, that job failed. Read its log before doing anything else.

There is no Intel macOS build: the matrix is Apple Silicon only, by the scope decision
in [`SCOPE.md`](SCOPE.md).

## 8. Install it somewhere real

A green build means it compiled and packaged. It does not mean it starts.

At minimum, install and launch on the platform you changed. The known risk is Windows:
`onnxruntime.dll` needs the Microsoft Visual C++ runtime (MSVCP140, VCRUNTIME140), and
a clean Windows without the redistributable does not have it. The installer does not
ship it.

## 9. Publish

Edit the draft, write what changed, and publish. GitHub adds the source archives at
that point, and `releases/latest` starts resolving, which is where the README's
download button points.

## Undoing a release

A draft is invisible until published, so before that point:

```sh
gh release delete v1.1.5 --yes
git push origin :refs/tags/v1.1.5
git tag -d v1.1.5
```

After publishing, delete the release and tag the same way and ship a new patch version.
Do not move a tag that people may already have fetched.

## Why the pipeline is shaped this way

Each of these is here because of a specific failure, not a preference.

- **Build does not run on pushes.** A twenty-five minute three-platform build on every
  push buries the five-minute checks that catch things.
- **Build does not cache `target/`.** A restored build directory lets cargo treat the
  native dependency's build script as fresh while the libraries it extracted are gone.
  The link then fails on a library that is no longer where the search path says it is:
  `unable to find library -lonnxruntime` on Linux, `cannot open input file 'cargs.lib'`
  on Windows. A release is built from clean on purpose.
- **Every workflow step runs under bash.** The default shell on Windows is pwsh, which
  propagates only the last command's exit code. A step that ran `cargo test` and then
  `git diff` reported success while the test binary was crashing.
- **`cross-check` runs clippy and nothing else.** It never links or runs a binary, so
  it is fast and cannot hit the loader problems that make running tests on Windows
  impractical. It exists to catch the one thing a Linux machine genuinely cannot see: a
  module whose callers are all behind `#[cfg(target_os = "linux")]` is dead code
  everywhere else, and `-D warnings` rejects it.
- **Tests do not run on Windows.** `sherpa-rs-sys` links the debug C runtime into every
  dev-profile binary there, so the test executable dies at load with
  `STATUS_ENTRYPOINT_NOT_FOUND`. Release builds never receive that directive, so it is
  a property of the test profile rather than of anything shipped.
