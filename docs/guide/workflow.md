---
layout: default
title: Workflows and project files
parent: Guide
nav_order: 2
description: "How FPVFlow combines video, motion data and a lens profile, how to save and reopen .gyroflow project files, how settings presets work, and how to process many clips in a batch."
---

# Workflows and project files
{: .no_toc }

1. TOC
{:toc}

---

## What FPVFlow needs for every clip

Gyro stabilization combines three things:

- **The video.**
- **Motion data**: how the camera rotated during the recording, measured by a gyroscope (and
  usually an accelerometer).
- **A lens profile**: the focal length and distortion of the lens at the recording mode you used.

FPVFlow lines the motion data up with the video (synchronization), computes a smoothed camera path,
and renders each frame as if the camera had followed that smooth path, correcting the lens
distortion on the way.

Where the motion data comes from decides how much work a clip takes.

### The camera records its own motion data

Many cameras write the gyro data into the video file: DJI air units and action cameras, GoPro
(HERO 5 and later), Insta360, Sony and others. The full list is in the
[Supported gyro sources](https://github.com/rodrigosclosa/fpvflow#supported-gyro-sources) section
of the README.

Open the video and FPVFlow reads the motion data from it. For the best-supported cameras it also
loads the matching lens profile on its own. Some cameras store motion data that is already in sync
with the frames. In that case no synchronization is needed, and FPVFlow folds the
**Synchronization** panel away to say so.

DJI footage is in this group. Two things to keep in mind:

- Record with RockSteady and HorizonSteady off. Gyro stabilization needs the unstabilized
  image.
- DJI O4 Pro and O4 Lite units can record false rotations ("Gyrogate"). FPVFlow detects this when
  the clip opens and can repair the motion data. See
  [DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}).

### The motion data comes from a separate device

If the camera has no gyro, another device can record the motion while it is rigidly attached to the
camera. For FPV the usual case is the flight controller: Betaflight blackbox logs (`.bbl`, `.bfl`,
`.csv`) and ArduPilot logs (`.bin`, `.log`) are supported. An action camera mounted on a bigger
camera, a phone app or a dedicated logger also work.

1. Open the video.
2. In **Motion data**, click **Open file** and choose the log.
3. If the log file contains several recordings, choose the right one in **Select log**.
4. If the logging device sits at an angle to the camera (camera uptilt on a quad), enter that angle
   in **Rotation**. See [Motion data]({{ '/guide/settings.html' | relative_url }}#motion-data).
5. Synchronize. The log and the video were started at different moments by different devices, so
   this step is required. See [Synchronization]({{ '/guide/synchronization.html' | relative_url }}).

## Project files (`.gyroflow`)

A `.gyroflow` file stores everything you set up for a clip: the paths to the video and the motion
data, the lens profile, the sync points, the stabilization and export settings, the trim ranges and
keyframes. In FPVFlow it also stores the external audio track and the LUT settings.

### Save a project

- Press <kbd>Ctrl</kbd>+<kbd>S</kbd> (<kbd>Cmd</kbd>+<kbd>S</kbd> on macOS). The first time, the
  project is saved next to the video with the same name and the `.gyroflow` extension. If such a
  file already exists, FPVFlow asks whether to overwrite it, rename the new one, or choose a
  different location. After that, the same shortcut overwrites the project you are working on.
- The arrow on the export button offers more variants:
  - **Export project file**: a small file that points to the motion data instead of embedding it.
    It needs the original files to stay where they are.
  - **Export project file (including gyro data)**: also embeds the motion data, so the project
    still works if the log is moved. This is what <kbd>Ctrl</kbd>+<kbd>S</kbd> saves.
  - **Save project file**: appears once the current clip has a project file, and saves over it.
- **Motion data** > **Export** > **Export project file (including processed gyro data)** adds the
  already computed camera orientations. Video editor plugins can use this file directly.

### Open a project

- Drop a `.gyroflow` file on the window, or open it like a video. FPVFlow loads the video it points
  to and restores all settings.
- When you open a video that has a `.gyroflow` file with the same name next to it, FPVFlow asks
  whether to load it.

{: .note }
Project files are also how you hand a clip to a video editor plugin (DaVinci Resolve, Premiere,
Final Cut and others). The plugins come from the Gyroflow project. Things that FPVFlow does during
its own export, such as the LUT and the external audio, are not applied by the plugins. See
[Exporting and the render queue]({{ '/export.html' | relative_url }}).

## Presets

A preset is a `.gyroflow` file that holds only the settings you choose, without a video. Use it to
reuse the same stabilization, sync or export settings on many clips.

### Create a preset

1. Set up a clip the way you like it.
2. Click the arrow on the export button and choose **Create settings preset**.
3. Tick the groups to include (for example **Stabilization** and **Export settings**, but not the
   **Offsets** or **Trim range**, which belong to one clip).
4. Choose where to store it:
   - **Save to file**: anywhere you like. Apply it later by dropping it on the window.
   - **Save to lens profile directory**: the preset then shows up in the **Lens profile** search
     box, tagged **preset**, and you apply it by picking it there.
   - **Save as default preset**: stored as `default.gyroflow` and applied automatically to every
     video you load.

{: .warning }
A default preset applies to every clip, including clips you open months later. If something looks
off on every clip, check whether a `default.gyroflow` is active. **Advanced** > **Open data folder**
shows where it lives (inside the `lens_profiles` folder).

A preset can only be applied after a video is loaded.

{: .note }
The preset dialog lists the Gyroflow settings groups. The LUT and external audio settings of
FPVFlow are not among them, so set those per clip.

## Processing many clips

FPVFlow inherits Gyroflow's render queue, which can prepare and export a whole flight session.

### Load the clips

- **Open several files at once** (select them in the file dialog, or drop them on the window).
  FPVFlow asks what to do: **Add to render queue**, **Merge them into one video** or **Open the
  first file**.
- **Drop files on the render queue** (press <kbd>Q</kbd> to show it). FPVFlow asks where to save
  the results: **Same as the original file** or a **Custom path**.

Each clip added this way gets the current export settings, its own lens profile if the camera is
recognized, and the default preset if you have one. It is not opened in the editor first, so check
what such jobs keep and what they miss in
[Exporting and the render queue]({{ '/export.html' | relative_url }}).

### Apply settings to all queued clips

- Drop a preset file on the render queue to apply it to every job in it.
- Or set up the current clip and choose **Apply selected settings to all items in the render
  queue** from the export button's arrow. You pick which groups to copy, as with a preset.

### Synchronize queued clips

Clips with built-in, already synced motion data (DJI, for example) need nothing. For clips that need
syncing, create a preset with **Synchronization** > **Do autosync** ticked and apply it. Queued
clips without sync points are then synchronized automatically before they render.

{: .tip }
Check one or two synced clips by eye before rendering a whole session. Auto sync can fail on clips
with little motion or very fast motion.

### Review clips one by one

While you edit a queued job, <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> saves it and opens the
next one, and <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> the previous one. Outside the queue,
<kbd>Ctrl</kbd>+<kbd>D</kbd> and <kbd>Ctrl</kbd>+<kbd>A</kbd> open the next and previous file in
the same folder. All shortcuts are on [Keyboard shortcuts]({{ '/guide/shortcuts.html' | relative_url }}).

### Project files instead of videos

In the queue's **Queue settings** > **Export mode** you can make the queue write project files
instead of videos, or both. Useful when you finish the edit in a video editor plugin.

How the queue runs, what each queued job keeps (LUT, external audio, Gyrogate setting) and how to
edit a job are described in [Exporting and the render queue]({{ '/export.html' | relative_url }}).

## Long flights split into several files

Some cameras split a long recording into several files. When FPVFlow recognizes such a set, it
offers to join them into one clip before you stabilize it.
