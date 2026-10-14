---
layout: default
title: Installation
description: "How to download and install FPVFlow on Windows, macOS and Linux, open the unsigned builds safely, check the system requirements and get updates."
nav_order: 2
---

# Installation
{: .no_toc }

FPVFlow is distributed only through its [GitHub Releases](https://github.com/rodrigosclosa/fpvflow/releases) page. It is not in any app store.
{: .fs-6 .fw-300 }

[Download the latest release](https://github.com/rodrigosclosa/fpvflow/releases/latest){: .btn .btn-primary }

1. TOC
{:toc}

---

## Pick your download

Each release has one file per platform:

| Platform | File |
|---|---|
| Windows x64 | `FPVFlow-windows64.zip` |
| macOS (Apple Silicon and Intel) | `FPVFlow-mac-universal.zip` |
| Linux x64 | `FPVFlow-linux64.AppImage` |

Android and iOS are not built by this fork. On mobile, use [Gyroflow](https://github.com/gyroflow/gyroflow).

## Windows

1. Download `FPVFlow-windows64.zip`.
2. Extract it to a folder of your choice.
3. Run `FPVFlow.exe`.
4. On the first run, SmartScreen warns that the app is unrecognized, because the build is not signed. Click **More info**, then **Run anyway**.

{: .note }
If you get an error about `VCRUNTIME140.dll` or `0xc0000142`, install the [Microsoft Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe) and run FPVFlow again.

## macOS

The macOS build is a universal binary: it runs natively on both Apple Silicon and Intel Macs.

1. Download `FPVFlow-mac-universal.zip` and unzip it.
2. Drag `FPVFlow.app` to your **Applications** folder.
3. Open FPVFlow. The first launch is blocked with "FPVFlow cannot be opened because Apple cannot verify it". Close that dialog.
4. Open **System Settings → Privacy & Security** and scroll down to the **Security** section. It says "FPVFlow was blocked to protect your Mac".
5. Click **Open Anyway**, confirm, and enter your password.

macOS remembers the choice, so this is needed only once.

{: .warning }
Right-click → Open no longer works for unsigned apps on macOS 15 (Sequoia) and later. Use **Open Anyway** in System Settings as described above.

If you prefer the terminal, you can remove the quarantine flag instead:

```sh
xattr -dr com.apple.quarantine /Applications/FPVFlow.app
```

## Linux

1. Download `FPVFlow-linux64.AppImage`.
2. Make it executable:

   ```sh
   chmod +x FPVFlow-linux64.AppImage
   ```

3. Run it.

Make sure you have the latest graphics drivers installed. On Debian and Ubuntu you may need these packages:

```sh
sudo apt install libva2 libvdpau1 libasound2 libxkbcommon0 libpulse0 libc++-dev libvulkan1
```

GPU-specific packages:

- **NVIDIA:** `nvidia-opencl-icd nvidia-vaapi-driver nvidia-vdpau-driver nvidia-egl-icd nvidia-vulkan-icd libnvcuvid1 libnvidia-encode1`
- **Intel:** `intel-media-va-driver i965-va-driver beignet-opencl-icd intel-opencl-icd`
- **AMD:** `mesa-vdpau-drivers mesa-va-drivers mesa-opencl-icd libegl-mesa0 mesa-vulkan-drivers`

## System requirements

- **Windows:** Windows 10 64-bit, version 1809 or later. On a Windows "N" edition, go to **Settings → Apps → Optional features → Add a feature** and enable **Media Feature Pack**.
- **macOS:** macOS 10.15 or later, Intel or Apple Silicon.
- **Linux:** the AppImage should work on any current distribution.

A GPU with up-to-date drivers makes the preview and the export much faster, but FPVFlow falls back to the CPU when GPU processing is not available.

## Why the builds are unsigned

The binaries are not code-signed on any platform. That is a cost decision, not an oversight:

| Platform | What signing costs | What you see without it |
|---|---|---|
| macOS | Apple Developer Program, US$ 99/year | **Open Anyway** in System Settings on first launch |
| Windows | Code signing certificate, about US$ 200-400/year | SmartScreen warning on first run |

For a free tool with no revenue, that is a yearly bill for removing two one-time clicks.

You do not have to take the binaries on trust. Every release is built by [GitHub Actions](https://github.com/rodrigosclosa/fpvflow/actions) from the commit it names, with the workflow file visible in the repository, and you can [build FPVFlow from source]({{ '/building.html' | relative_url }}) yourself.

If donations ever cover the certificates, signing is the first thing they go to, starting with macOS.

## Using FPVFlow alongside Gyroflow

FPVFlow and Gyroflow can be installed side by side. FPVFlow reads lens profiles and preferences from the same directory Gyroflow uses, so an existing Gyroflow setup carries over.

To check which app you are running, look at the version shown in the app: FPVFlow shows "v1.0.x · based on Gyroflow 1.6.3".

## Updating

Every time it starts, FPVFlow checks its own GitHub releases for a newer version. When one is available, it shows "There's a newer version available" with the release notes. Click **Download** to open the releases page, or **Close** to dismiss the notice.

To update:

1. Download the new file for your platform.
2. Replace the old app with the new one (on Windows, extract over the old folder or into a new one; on macOS, replace `FPVFlow.app` in Applications; on Linux, replace the AppImage).
3. On macOS, repeat the **Open Anyway** step the first time you open the new version, if macOS blocks it again.

Your settings, lens profiles and LUT folder are kept, because they are not stored inside the app.
