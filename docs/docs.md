---
layout: default
description: "FPVFlow is a free Gyroflow fork for FPV pilots: gyro-based video stabilization with a DJI O4 Gyrogate repair, .cube LUT colour grading and an external audio track that syncs automatically."
title: Documentation
nav_order: 1
permalink: /docs/
---

# FPVFlow
{: .fs-9 }

Gyro-based video stabilization for FPV pilots, with colour grading and external audio built in.
{: .fs-6 .fw-300 }

[Download FPVFlow](https://github.com/rodrigosclosa/fpvflow/releases/latest){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[Installation guide]({{ '/installation.html' | relative_url }}){: .btn .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/rodrigosclosa/fpvflow){: .btn .fs-5 .mb-4 .mb-md-0 }

---

**FPVFlow is a free, open-source fork of [Gyroflow](https://github.com/gyroflow/gyroflow) 1.6.3 built for FPV pilots.** It stabilizes your flight footage from the camera's own gyro data, exactly like Gyroflow, and adds three things FPV footage needs: a repair for the **DJI O4 "Gyrogate"** false rotations, **colour grading with `.cube` LUTs** applied during the export, and an **external audio track** (a DJI Mic, a recorder on the pilot) that **syncs to the video automatically**. One pass, one file, ready to post.

It runs on Windows, macOS (Apple Silicon and Intel) and Linux, and is licensed under GPL-3.0.

![FPVFlow main window with a clip loaded, the stabilized preview, the timeline with the gyro chart and the side panels]({{ '/assets/images/screenshot.png' | relative_url }})

<p align="center">
  <img src="https://raw.githubusercontent.com/rodrigosclosa/fpvflow/fpvflow/resources/comparison1.gif" alt="Side-by-side comparison of raw and stabilized FPV footage" height="200" loading="lazy">
  <img src="https://raw.githubusercontent.com/rodrigosclosa/fpvflow/fpvflow/resources/comparison2.gif" alt="Second side-by-side comparison of raw and stabilized FPV footage" height="200" loading="lazy">
</p>

## What FPVFlow adds

Each feature was built for a specific FPV problem: you land with log footage and a separately recorded audio track and do not want a second pass through an editor, or you land with DJI O4 footage whose gyro recorded rotations that never happened.

### DJI O4 "Gyrogate" repair

DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use a gyro that records short false rotations under motor vibration. The raw video looks fine, but once stabilized it jerks and slowly sways. FPVFlow detects the defect when the clip loads, turns on **Remove DJI Gyrogate false rotations** for affected clips, and can **Save corrected copy** of the file for use in other tools.

[Read about the Gyrogate repair]({{ '/gyrogate.html' | relative_url }})

### Colour grading with `.cube` LUTs

Load a conversion LUT for D-Log M, GoPro Flat or any other log profile and FPVFlow applies it **during stabilization**, so the exported file is already graded. Adjust the intensity, keep a LUT library folder, and fine-tune with 13 colour adjustments. 10-bit footage stays 10-bit.

[Read about colour grading]({{ '/colour-grading.html' | relative_url }})

### External audio with auto-sync

Import a WAV, M4A, MP3, FLAC or AAC file and FPVFlow embeds it in the stabilized export, in its original format where the container allows it. **Auto-sync audio** lines it up for you, using the propeller vibration heard by the microphone and read by the gyro, or, on DJI O4 footage, the arming beep and the disarm. The track plays in the preview and shows as a waveform on the timeline, so you can check the sync by ear and by eye.

[Read about external audio]({{ '/external-audio.html' | relative_url }})

## Everything Gyroflow does

FPVFlow adds to Gyroflow rather than replacing it. Everything from Gyroflow 1.6.3 still works, including:

- Real-time preview and GPU-accelerated rendering
- Rolling shutter correction and lens correction, with the built-in lens profile database
- Support for GoPro, DJI, Insta360, Sony, RunCam and many other gyro sources, plus Betaflight blackbox logs
- Several smoothing algorithms, horizon levelling, adaptive zoom and keyframes
- Render queue and settings presets
- H.264, H.265, ProRes, DNxHD, CineForm, PNG and OpenEXR output, with 10-bit support

The lens profiles, the `.gyroflow` project format and the [Gyroflow documentation](https://docs.gyroflow.xyz) all still apply. For general topics such as lens profiles, synchronization and smoothing, the Gyroflow docs are the reference.

{: .important }
**FPVFlow is not an official Gyroflow release.** Please do not report FPVFlow problems to the Gyroflow project; open them in the [FPVFlow issue tracker](https://github.com/rodrigosclosa/fpvflow/issues) instead. All credit for the stabilization engine belongs to the Gyroflow authors. If you value it, consider [supporting Gyroflow](https://github.com/sponsors/AdrianEddy) too.
