---
layout: default
title: Building from source
description: "Build FPVFlow from source on Windows, macOS and Linux: toolchain, code structure, dev environment, the pinned MDK SDK, contributing and translations."
nav_order: 12
---

# Building from source
{: .no_toc }

This page is for developers. To use FPVFlow, download a ready build from the
[releases page](https://github.com/rodrigosclosa/fpvflow/releases/latest) instead.

1. TOC
{:toc}

---

## Languages and technologies

FPVFlow, like Gyroflow, is written in [Rust](https://www.rust-lang.org/), with the UI in
[QML](https://doc.qt.io/qt-6/qmlfirststeps.html). The application depends on *Qt*, *FFmpeg*,
*OpenCV* and *mdk-sdk*; the core library is pure Rust with no external dependencies.

- **OpenCV** is kept to a minimum: only lens calibration and parts of the synchronization use it
  (`src/core/calibration/mod.rs` and, in `src/core/synchronization/`, the optical-flow backends
  `optical_flow/opencv_dis.rs` and `optical_flow/opencv_pyrlk.rs`, the pose estimators in
  `estimate_pose/` and `find_offset/visual_features.rs`). The core algorithms and the
  undistortion do not.
- **GPU**: DirectX, OpenGL, Metal and Vulkan through *Qt RHI* and *wgpu*. Processing runs on
  *OpenCL* or *wgpu*, with a multi-threaded CPU implementation as the fallback.
- **Qt 6.7.3** is used on every platform (upstream still defaults x86_64 desktop to 6.4.3). The
  compiled Qt RHI shaders in `src/qt_gpu/compiled` are one set of `.qsb` files shared by all
  platforms, and the `.qsb` format is versioned, so every platform has to use the same Qt.

{: .note }
The FPVFlow binary links against mdk-sdk, which is closed-source and not licensed under the GNU
GPL. FPVFlow, like Gyroflow, is licensed under GPLv3 with an additional permission allowing
linking with mdk-sdk. mdk-sdk is distributed under its own license.

## Code structure

1. `src/ui` holds the entire GUI (QML). The side panels are in `src/ui/menu`.
2. `src/controller.rs` is the bridge between UI and core: it takes every command from QML and
   calls into the core.
3. `src/core` is the stabilization engine. It does not depend on Qt or FFmpeg; OpenCV is
   optional. The Gyrogate repair lives in `src/core/gyrogate`.
4. `src/rendering` holds all the FFmpeg code, for rendering the final video and for processing
   used by synchronization.
5. `src/core/gpu` holds the GPU implementations of the undistortion.
6. `src/qt_gpu` is the zero-copy GPU undistortion path, using Qt RHI and a GLSL compute shader.
7. `src/fpvflow.rs` is the main entry point.
8. `mod.rs` or `lib.rs` in each directory is the entry of that module (the directory name is the
   module name).

{: .note }
The crates are renamed to `fpvflow`. Code and docs inherited from upstream may still say
`gyroflow_core` or `gyroflow.rs`; in this repository they are the `fpvflow` equivalents.

## Dev environment

- **Visual Studio Code** with the `rust-analyzer` extension.
- **Qt Creator** for QML: load the QML files there for auto-complete and syntax highlighting.
- **UI live reload**: set `ui_live_reload` to `true` in `src/fpvflow.rs`. From then on, every
  change to a QML file is reloaded in the running app immediately.

## Building on Windows

1. Install the prerequisites: `git`, `7z` and a working `powershell`. If you have never run
   PowerShell scripts, run `set-executionpolicy remotesigned` in PowerShell as administrator.
2. Install the latest stable Rust from [rustup.rs](https://rustup.rs/). When the Visual Studio
   Installer adds the C++ build tools, check the "English language pack" option.
3. Install Just: `cargo install --force just`
4. Clone the repository:
   ```
   git clone https://github.com/rodrigosclosa/fpvflow.git
   ```
5. In the project directory, install the dependencies: `just install-deps`
6. Compile and run: `just run`

## Building on macOS

1. Install the prerequisites: `git` and `brew`.
2. Install the latest stable Rust from [rustup.rs](https://rustup.rs/).
3. Install Just: `cargo install --force just`
4. Clone the repository:
   ```
   git clone https://github.com/rodrigosclosa/fpvflow.git
   ```
5. In the project directory, install the dependencies: `just install-deps`
6. Run `just deploy` once. The first `just run` does not work without it.
7. Compile and run: `just run`

## Building on Linux

1. Install the prerequisites: `git`, `7z`, `python` and the `apt` package manager. On another
   distribution, adjust the commands inside the scripts in `_scripts/`.
2. Install the latest stable Rust from [rustup.rs](https://rustup.rs/).
3. Install Just: `cargo install --force just`
4. Clone the repository:
   ```
   git clone https://github.com/rodrigosclosa/fpvflow.git
   ```
5. In the project directory, install the dependencies: `just install-deps`
6. Compile and run: `just run`

## Android and iOS

The `just android ...` and `just ios ...` recipes (`_scripts/android.just`, `_scripts/ios.just`)
are inherited from Gyroflow, but FPVFlow does not build or test them. Use
[Gyroflow](https://github.com/gyroflow/gyroflow) on mobile.

## The MDK SDK is pinned

The video player in the preview is [mdk-sdk](https://github.com/wang-bin/mdk-sdk), pulled in
through a fork of `qml-video-rs`
([rodrigosclosa/qml-video-rs](https://github.com/rodrigosclosa/qml-video-rs)), referenced by
commit in `Cargo.toml`. The SDK version is fixed in that fork's `build.rs`, always to a **tagged
GitHub release** of `wang-bin/mdk-sdk`: today **v0.38.0** (git `6c92f5a`).

{: .warning }
Never point it at the "latest" build on SourceForge. That is a rolling build: the rolling 0.38.0
of 2 September 2026 (git `744fc5b`) broke the external audio track with no visible error. The
video played, the audio did not, and the log showed `read packet error` and `seekTo` jumping
backwards. The tagged v0.38.0 release plays the same files normally.

{: .note }
The Windows package is `mdk-sdk-windows-clang.7z`. The `mdk-sdk-windows-desktop-clang.7z` still
published with the releases is a legacy package that contains 0.35.1.

### Validating an MDK update

{: .important }
Any MDK SDK update requires re-validating the external audio track in the preview before it is
accepted.

1. Change the release in the `qml-video-rs` fork's `build.rs`, update the commit in `Cargo.toml`
   and build locally.
2. Load a video and a WAV file as its external audio track.
3. Run the audio auto-sync.
4. Press play and confirm the audio plays.
5. Drag the playhead to another point and press play again.
6. In the log, confirm the line `[MDK] <version> (git <hash>)` shows the version you expect, and
   that the audio kept playing after the seek. `MDKPlayer.cpp` logs every seek with the result
   MDK returned, and every play and pause.

If the audio breaks, compare first with a C++ harness built directly on the SDK, running the same
sequence (`setMedia` → `setMedia(wav, Audio)` → `setActiveTracks` → `prepare` → seek → play),
before blaming the app.

## Profiling

### On Windows

1. Install and run Visual Studio Community Edition.
2. Compile and run with the `profile` profile: `just profile`
3. In Visual Studio, open `Debug -> Performance Profiler...`.
4. Under `Target`, open `Change Target`, choose `Running Process...` and select the running
   `fpvflow.exe` process.

### QML

1. Uncomment `config.define("QT_QML_DEBUG", None);` in `build.rs`.
2. Comment out the `cli::run(...)` check in `src/fpvflow.rs`.
3. Run in debug mode with the QML debugger arguments:
   ```
   cargo run -- "-qmljsdebugger=port:1234,block,services:CanvasFrameRate,EngineControl,DebugMessages"
   ```
4. In Qt Creator, open `Analyze` -> `QML Profiler (Attach to Waiting Application)` and enter
   port 1234.

## Contributing

- **Bugs and feature ideas** for FPVFlow go to the
  [FPVFlow issue tracker](https://github.com/rodrigosclosa/fpvflow/issues). A sample clip makes a
  bug report far more useful. Please do not take FPVFlow problems to the Gyroflow project.
- **Code**: fork the repository, work on a branch from `fpvflow` and open a pull request.

New to the stack? The [Rust book](https://doc.rust-lang.org/book/), [quickref.me/rust](https://quickref.me/rust),
[cheats.rs](https://cheats.rs/) and the [Qt QML book](https://www.qt.io/product/qt6/qml-book) are
good starting points.

## Translations

The UI translations are the `.ts` files in `resources/translations`, compiled to `.qm` files that
are embedded in the binary.

- **Gyroflow's own strings** are translated upstream on
  [Crowdin](https://crowdin.com/project/gyroflow) and arrive in FPVFlow when upstream is merged.
- **Strings added by FPVFlow** (the colour panel, the external audio panel and the Gyrogate
  section of the motion data panel) are not in that Crowdin project, so translating them there
  does not reach FPVFlow.
  [Open an issue](https://github.com/rodrigosclosa/fpvflow/issues) if you want to translate them.
