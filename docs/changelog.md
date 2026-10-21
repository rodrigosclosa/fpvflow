---
layout: default
title: Changelog
description: "What changed in each FPVFlow release, newest first: the Gyrogate tremor repair, the DJI O4 Gyrogate repair, the beep-anchored audio auto-sync, faster LUT exports and more."
nav_order: 13
---

# Changelog
{: .no_toc }

Newest release first. Every release is on the
[GitHub releases page](https://github.com/rodrigosclosa/fpvflow/releases), with downloads for
Windows, macOS and Linux.

1. TOC
{:toc}

---

## 1.0.4

[Release page](https://github.com/rodrigosclosa/fpvflow/releases/tag/v1.0.4){: .btn .btn-primary }

### Gyrogate: the single-axis tremor is smoothed too

Besides the short false rotations, the ICM-40609-D gyro of DJI O4 units records a second, quieter
defect: a dense tremor on a single axis, usually roll. One axis carries many small spikes per
second while the other two stay calm. Each spike is too small to count as a false rotation, but
together they make the stabilized picture shiver.

**Remove DJI Gyrogate false rotations** now smooths that tremor as well, on the affected axis
only and only where the spikes pile up. The other axes and the calm parts of the clip are not
touched.

- On the public O4 Pro sample clip, which 1.0.3 reported as clean, the roll jitter drops from
  0.072° to 0.017° per frame.
- On a real 2026 O4 Pro flight, the tremor drops by about half where it was strongest. The calm
  stretches are unchanged.

The **Motion data** panel shows how many seconds of tremor the clip has, and the repair now
also turns on for clips with tremor only. **Sensitivity** moves the tremor floor along with the
false-rotation threshold, and **Save corrected copy** includes the tremor repair.

Jolts or jello that you can already see in the raw footage are real camera motion, not
Gyrogate, and no motion-data repair can remove them. See
[Limits]({{ '/gyrogate.html#limits' | relative_url }}).

### Colour panel in your language

The **Color** panel labels follow the app language: English by default, Portuguese with the
pt_BR language. The About tagline is in English too: "Gyroflow enhanced for FPV pilots".

### Smaller Windows download

The 1.0.3 Windows zip held every file twice, once at the top and once in a `win64` folder. The
1.0.4 zip holds each file once, so the download is about half the size.

### Website and documentation

FPVFlow now has this website, with the full documentation.

---

## 1.0.3

[Release page](https://github.com/rodrigosclosa/fpvflow/releases/tag/v1.0.3){: .btn .btn-primary }

### DJI O4 "Gyrogate" repair

DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use the TDK
ICM-40609-D gyro. Under motor vibration it records short false rotations on a single axis - up to
about 2° in a few milliseconds - and the camera then slowly pulls each one back over about 12
seconds. The raw footage looks fine, but the stabilized video jerks and sways, because the
stabilization corrects rotations that never happened. FPVFlow now detects and repairs this from
the motion data alone.

- **Detection when the clip loads.** The **Motion data** panel opens by itself on an affected
  clip and shows which gyro the unit most likely has (read from the model and from the production
  date in its serial number) and how many false rotations the clip contains.
- **Repair in the stabilization.** **Remove DJI Gyrogate false rotations** is turned on
  automatically for affected clips and saved with the project and the render queue. It removes
  each false burst, compensates the camera's slow pull-back and smooths the result. Clips without
  false rotations are left untouched.
- **Sensitivity.** A **Sensitivity** slider, with a help icon that explains it, trades catching
  smaller false rotations against touching real knocks. The default of 50% suits most clips.
- **Corrected copy.** **Save corrected copy** writes `<name>_gyrofix.MP4` next to the original,
  with only the attitude data rewritten: same size, video and audio untouched, the original never
  modified. FPVFlow then uses it as the motion data and keeps your sync, lens, audio and colour
  settings. The copy opens like any DJI clip in FPVFlow or Gyroflow.

{: .important }
The repair needs the 1 kHz attitude track, so record with RockSteady and HorizonSteady off. It
removes most of the false motion, not all of it; the real fix has to come from a DJI firmware
update.

On O4 Pro units with the original MP66 gyro the repair stays off: the short jolts it may find
there are usually real knocks, such as setting the drone down.

The algorithm is a port of [o4fix](https://github.com/vadimbryl/o4fix) by vadimbryl, and its
output matches o4fix exactly on the public sample clips. Thanks also to
[DJIFix](https://github.com/parhipov/DJIFix) and [djgyrofix](https://github.com/steamvogue/djgyrofix)
for their analyses and sample footage.

### Documentation

The README now describes the Gyrogate repair, the new audio auto-sync and the faster LUT
exports, and no longer says that external audio is silent in the preview - it plays there.

## 1.0.2

[Release page](https://github.com/rodrigosclosa/fpvflow/releases/tag/v1.0.2){: .btn .btn-primary }

### Much more accurate audio auto-sync

The auto-sync used to match the take-off heard in the audio against the lift-off seen in the
gyro. On DJI O4 Pro footage the gyro only reports orientation about twice a second, so the result
was often several seconds off, and worse with loud background noise. It now anchors on sounds the
drone makes at precise moments:

- **The arming beep.** The three-tone beep the drone plays when it arms is found in the audio,
  even with a crowd or music around. For a camera set to record on arm, the first frame follows
  the beep by a fixed delay.
- **The disarm.** The propellers stopping at the end of the flight marks the last frame of a
  recording that stops on disarm. Both ends must agree before the result is accepted.
- **Recordings started by hand.** Carrying the drone into place is no longer taken for the
  take-off: the sync uses the moment the drone leaves its rest on the ground, cross-checked
  against the disarm, so a recording stopped by hand mid-flight is also detected.

On ten hand-aligned test clips, the result is within 0.15 s for recordings that run from arm to
disarm, and within about 0.5 s (1.35 s at worst) for recordings started by hand.

{: .note }
A recording that is both started and stopped by hand and has no arming beep still falls back to
the older, less accurate method. Check that one by hand.

### Faster exports with a LUT or colour adjustments

Exports with a LUT or colour adjustments went from about 7 fps to about 30 fps on 4K footage
on a Mac with an M4 Pro: the colour conversion now uses all CPU cores and a faster filter. For the previous, slightly more
precise conversion, enable **Precise color conversion (slower export)** in the **Color** panel.

### Fixes

- **Freeze when playing with external audio while the queue renders.** The app no longer hangs.
- **Queue lost the audio and the colour grading.** Reopening the app or a queued job now restores
  each clip's external audio, its sync offset, and its LUT and colour adjustments.
- **Editing a queued job showed "Export" instead of "Save".** Editing a job from the render queue
  now saves it back to the queue.
- **Update notice.** The check for a new version compares release tags, so it points to FPVFlow
  releases correctly.

### Dark app icon

The app icon is now the FPVFlow mark on a dark tile on every platform. On macOS it was a light
tile, and the Linux AppImage still had the Gyroflow icon.

## 1.0.1

[Release page](https://github.com/rodrigosclosa/fpvflow/releases/tag/v1.0.1){: .btn .btn-primary }

A maintenance release with no new features. The video player library used by the preview (MDK
SDK) moves to its tagged v0.38.0 release; 1.0.0 had been held on 0.35.1 because a rolling 0.38.0
build stopped the external audio track from playing in the preview. The release notes also gained
the current way to open the unsigned app on macOS 15 (Sequoia) and later.

## 1.0.0

[Release page](https://github.com/rodrigosclosa/fpvflow/releases/tag/v1.0.0){: .btn .btn-primary }

The first release of FPVFlow, a fork of [Gyroflow](https://github.com/gyroflow/gyroflow) 1.6.3
with features aimed at FPV pilots. Everything Gyroflow does, it still does.

### Colour grading with `.cube` LUTs

- Load any Adobe Cube (`.cube`) LUT, 1D or 3D, and apply it during stabilization, so the exported
  file is already graded.
- Intensity slider to blend the grade against the original.
- LUT library: point it at a folder once and your LUTs are two clicks away.
- 13 colour adjustments applied after the LUT: exposure, luminance, contrast, highlights,
  shadows, whites, blacks, temperature, tint, saturation, vibrance, sharpness and vignette.
- The output is tagged Rec.709 when a LUT is loaded.

### External audio track

- Import a separately recorded audio file and export it embedded in the stabilized video.
- Auto-sync that pairs the take-off heard in the audio with the lift-off seen in the motion data.
- Waveform on the timeline, below the gyro chart, and playback in the preview.

### Other

- The update check looks at FPVFlow releases.
- Lens profiles and preferences are read from the same directory Gyroflow uses, so an existing
  install carries over. Both apps can be installed side by side.
