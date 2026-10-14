---
layout: default
title: Getting started
description: "The FPVFlow workflow for a typical FPV clip: open the video, check the lens profile and motion data, synchronize, set the stabilization, preview and export."
nav_order: 3
---

# Getting started
{: .no_toc }

This page takes one FPV clip through FPVFlow, from opening the file to the stabilized export. Each
step points to the Gyroflow documentation when you need more depth, and to the FPVFlow pages for
the features this fork adds.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## The window at a glance

FPVFlow keeps the Gyroflow layout. The panels are collapsible sections, laid out in the order you
normally work through them:

| Where | Panels |
|---|---|
| Left column | **Video information**, **Lens profile**, **Motion data**, **External audio** |
| Centre | The preview, and below it the timeline with the motion data chart |
| Right column | **Synchronization**, **Stabilization**, **Color**, **Export settings**, **Advanced** |
| Bottom bar | **Output path:**, the export button and the **Render queue** button |

**External audio** and **Color** are FPVFlow panels, and so is the Gyrogate section inside
**Motion data**. Everything else works exactly as in Gyroflow 1.6.3.

{: .note }
FPVFlow reads lens profiles and preferences from the same folder as Gyroflow, so an existing
Gyroflow install carries over. Both apps can be installed side by side.

## 1. Open the video

1. Drag the clip onto the window (the preview shows **Drop video file here**), or click **Open file** in
   the **Video information** panel.
2. Wait while FPVFlow analyzes the file and loads its motion data.
3. Check **Video information**: it lists the **Detected camera**, **Detected lens**, **Frame rate**
   and whether the file **Contains gyro**.

FPVFlow may ask a question while the clip loads:

- **Split recording has been detected** - the camera split a long flight into several files. Answer
  **Yes** to join them into one clip.
- "There's a *clip*.gyroflow file associated with this video, do you want to load it?" - you saved
  a project for this clip before. Answer **Yes** to continue where you left off.

{: .tip }
Record with the camera's own stabilization off (RockSteady, HorizonSteady, HyperSmooth and
similar). Gyroflow needs the raw, unstabilized image to work with. For DJI O4 units this also
matters for the [Gyrogate repair]({{ '/gyrogate.html' | relative_url }}), which needs the 1 kHz
attitude track that is only recorded with RockSteady and HorizonSteady off. See
[Common filming tips and issues](https://docs.gyroflow.xyz/app/getting-started/common-filming-tips-and-issues)
for more.

## 2. Check the lens profile

The lens profile tells FPVFlow how your lens distorts the image. Without it the stabilization
cannot be correct.

1. Open the **Lens profile** panel. For many cameras the profile is loaded automatically from the
   built-in database.
2. If nothing is loaded, type your camera model in the search field and pick the profile that
   matches your camera, lens and recording resolution.
3. Watch for the warnings under the profile: a mismatch between the profile and the file
   dimensions or aspect ratio means the result will not look right.

If no profile exists for your camera, you can make one with **Create new** (a chessboard
calibration). The Gyroflow docs cover both topics:
[Lens profiles](https://docs.gyroflow.xyz/app/advanced-usage/lens-profiles) and
[Lens calibration](https://docs.gyroflow.xyz/app/getting-started/lens-calibration).

## 3. Check the motion data

The **Motion data** panel shows where the gyro data comes from: the **File name** and the
**Detected format**. The timeline under the preview draws the same data as a chart.

**Data recorded by the camera** (GoPro, DJI, Insta360 and others) is loaded with the video. There
is nothing to do here unless the chart looks wrong.

**Data from an external source**, such as a Betaflight blackbox log for a camera without a gyro:

1. Click **Open file** in **Motion data** and choose the log (`.bbl`, `.bfl`, `.csv` and the other
   supported formats).
2. If the file contains several flights, pick the right one in **Select log**.
3. Turn on **Rotation** and set the camera angle. **Pitch** is the camera uptilt relative to the
   flight controller. The values have no effect while **Rotation** is off.
4. If the horizon or the motion direction is wrong, check **IMU orientation**.

The Gyroflow page on
[using an external gyro source](https://docs.gyroflow.xyz/app/advanced-usage/using-external-gyro-source)
explains rotation and orientation in detail.

### DJI O4 clips and the Gyrogate section

For DJI O4 clips the panel has an extra section. It shows which gyro your unit most likely has,
read from the model and the production date in its serial number, and how many false rotations
the clip contains. When the clip is affected, the **Motion data** panel opens by itself and
**Remove DJI Gyrogate false rotations** is already turned on.

The [Gyrogate page]({{ '/gyrogate.html' | relative_url }}) explains the repair, the
**Sensitivity** slider and **Save corrected copy**.

The other filters in this panel (**Low pass filter**, **Median filter**, **Glitch filtering**) come
from Gyroflow and are not needed for most clips.

## 4. Synchronize

The gyro data and the video must be aligned in time.

**If the camera recorded its own orientation in sync with the video**, as DJI units do, there is
nothing to synchronize. FPVFlow collapses the **Synchronization** panel for these clips, because
extra sync points are not needed and can make the result worse.

**If the motion data needs syncing**, for example a blackbox log or a camera that records raw
gyro:

1. Make sure the lens profile is loaded first. Synchronization without it will most likely give
   wrong results, and FPVFlow warns you about it.
2. In **Synchronization**, click **Auto sync**. FPVFlow analyzes several points along the clip and
   places sync points on the timeline.
3. Play the clip with stabilization on and check that it looks steady from start to end.

For the sync settings and how to fix a bad sync point, see
[Synchronization](https://docs.gyroflow.xyz/app/advanced-usage/synchronization) in the Gyroflow
docs.

### Separate audio track

If you recorded sound separately (a DJI Mic on the pilot, or a camera without a microphone):

1. In **External audio**, click **Import external audio** and choose the file (WAV, M4A, MP3,
   FLAC, AAC).
2. Click **Auto-sync audio**. FPVFlow looks for sounds the drone makes at precise moments, such as
   the arming beep and the disarm.
3. Play the clip and check the alignment by ear. The waveform on the timeline helps you see it.
   Adjust **Offset** by hand if needed.

Details are on the [External audio page]({{ '/external-audio.html' | relative_url }}).

## 5. Set the stabilization

The **Stabilization** panel controls how smooth the result is and how much of the frame is
cropped.

- **Smoothing.** Choose the algorithm and the **Smoothness**. Higher values give a smoother, more
  cinematic look; lower values keep more of the original motion.
- **Lock horizon.** Keeps the horizon level. Useful for cruising footage; usually off for freestyle
  where the rolls are the point.
- **Zoom.** Stabilizing needs some cropping. **Dynamic zooming** adjusts the crop over time, and
  **Zoom limit** caps how far it can go. **FOV** sets the overall framing.

The Gyroflow page on [Stabilization](https://docs.gyroflow.xyz/app/advanced-usage/stabilization)
describes every algorithm and parameter.

### Grade the image

If you shot in a log profile (DJI D-Log M, GoPro Flat and similar), open the **Color** panel,
click **Load LUT** and choose the conversion `.cube` file. Use **Intensity** to blend it with the
original. The grade is applied during the stabilization render, so the exported file is already
graded. See [Colour grading]({{ '/colour-grading.html' | relative_url }}).

## 6. Preview

The preview shows the stabilized result in real time.

| Action | Key |
|---|---|
| Play / pause | <kbd>Space</kbd> |
| Stabilization on/off, to compare with the original | <kbd>S</kbd> |
| Stabilization overview (shows the crop on the full frame) | <kbd>D</kbd> |
| Mute | <kbd>M</kbd> |
| Set the trim start / end at the playhead | <kbd>I</kbd> / <kbd>O</kbd> |
| Clear the trim range | <kbd>C</kbd> |

The external audio plays in the preview, aligned with the video, so you can check the sync while
you watch. If playback is slow on your computer, lower **Preview resolution** in the **Advanced**
panel. It only affects the preview, not the export.

The full list of shortcuts is in the Gyroflow docs:
[Keyboard shortcuts](https://docs.gyroflow.xyz/app/getting-started/keyboard-shortcuts).

## 7. Export

1. Check the file name and folder in **Output path:** at the bottom.
2. In **Export settings**, pick the codec and the output size. **Export audio** must be on for the
   external audio track to be embedded.
3. Click **Export** (<kbd>Ctrl</kbd>+<kbd>W</kbd>), or add the clip to the render queue
   (<kbd>Ctrl</kbd>+<kbd>Q</kbd>) and move on to the next one.

On macOS, use <kbd>Cmd</kbd> wherever this documentation says <kbd>Ctrl</kbd>.

FPVFlow warns you before exporting if the lens profile is missing or the clip has no sync points.
The [Exporting and the render queue]({{ '/export.html' | relative_url }}) page covers the export settings, batch
rendering and what each queued job keeps.

{: .tip }
Press <kbd>Ctrl</kbd>+<kbd>S</kbd> to save a `.gyroflow` project next to the video. The next time
you open the clip, FPVFlow offers to load it with all your settings.
