---
layout: default
title: Panel reference
parent: Guide
nav_order: 3
description: "Reference of every FPVFlow panel and control: Video information, Lens profile, Motion data, Synchronization, Stabilization, Export settings and Advanced, with defaults and when to change them."
---

# Panel reference
{: .no_toc }

Every panel of the main window, control by control. Most controls also explain themselves in a
tooltip when you hover over them.

1. TOC
{:toc}

---

## Layout and common controls

| Where | Panels |
|---|---|
| Left column | **Video information**, **Lens profile**, **Motion data**, **External audio** |
| Centre | Preview, playback controls, timeline with the motion chart |
| Right column | **Synchronization**, **Stabilization**, **Color**, **Export settings**, **Advanced** |
| Bottom bar | **Output path:**, the export button and the **Render queue** button |

Click a panel title to fold or unfold it. Several panels hide less common controls under an
**Advanced** heading at the bottom.

**Sliders and number fields.** Right-click a slider for **Reset value**, which returns it to its
default. Controls that can change over time also offer **Enable keyframing** and **Add keyframe** in
the same menu. With keyframing on, every change you make is stored at the playhead position, and the
keyframes appear on the timeline. Right-click a keyframe there to **Delete** it or to choose how
FPVFlow moves into and out of it (**Ease in**, **Ease out**, **Smooth**).

## Video information

Shows what FPVFlow read from the clip: **File name**, **Detected camera**, **Detected lens**,
**Dimensions**, **Duration**, **Frame rate**, **Codec**, **Pixel format**, **Audio**, **Rotation**
and **Contains gyro**, plus **Created at** and lens or exposure metadata when the file has them.

**Open file** opens a video, an image sequence or a `.gyroflow` project.

Two values can be edited with the pencil icon next to them:

- **Frame rate**: overrides the frame rate stored in the file. FPVFlow rescales the timing of the
  motion data and the keyframes by the ratio between the new and the original value, and sets
  **Analyze every n-th frame** in **Synchronization** to that ratio (rounded down). Use it when the
  container's frame rate is not the one the sensor ran at.
- **Rotation**: starts at the rotation read from the file's metadata. Enter another angle, in
  degrees, to turn the picture. It can be keyframed.

## Lens profile

Selects the lens profile for the clip. How profiles work and how to make one is explained in
[Lens profiles and calibration]({{ '/guide/calibration.html' | relative_url }}).

| Control | What it does |
|---|---|
| Search box | Finds profiles (and presets saved in the lens profile folder) by camera, lens or mode. The star marks favourites. |
| **Open file** | Loads a profile `.json` from disk. |
| **Create new** | Opens the lens calibrator. |
| Profile table | **Camera**, **Lens**, **Setting**, **Additional info**, **Dimensions**, **Calibrated by** and more, for the loaded profile. |

Warnings under the search box tell you when the profile is unofficial (with a **Good** / **Bad**
rating link) and when its size or aspect ratio does not match the video.

Under **Advanced**:

| Control | What it does |
|---|---|
| **Lens is under water** | Corrects for water refraction. Can be keyframed from its right-click menu, for clips that go in and out of the water. |
| **Pixel focal length**, **Focal center**, **Distortion coefficients** | The profile's numbers. Edit them to fine-tune a profile that is almost right. |
| **Export STMap** | Writes the lens distortion as an STMap image for compositing software. For files with per-frame lens data it can write one map or a sequence. |

## Motion data

Loads and conditions the gyro data. Supported formats are listed under
[Supported gyro sources](https://github.com/rodrigosclosa/fpvflow#supported-gyro-sources) in the
README.

| Control | What it does |
|---|---|
| **Open file** | Loads motion data from a separate file: a blackbox or ArduPilot log, a `.gcsv`, a CSV or app log, or another camera's video. |
| **File name** / **Detected format** | Which file the motion data comes from and how it was recognized. |
| **Select log** | Appears when the file holds several recordings, such as a blackbox file with several arms. Pick the one that belongs to the video, or **All logs combined**. |
| **Load all metadata** | Appears for a separate motion data file. Reads all of that file's metadata, as it would for the main video, instead of only the motion data. |
| **Frame offset** | Appears with the previous option. Shifts the video by whole frames against the motion data. |
| **Low pass filter** | Smooths high-frequency noise out of the gyro data. The cutoff is in Hz (50 Hz when you first enable it); a lower cutoff filters more. Useful for noisy blackbox logs. |
| **Median filter** | Removes isolated spikes, over a window of samples. |
| **Glitch filtering** | Finds and repairs short bursts of corrupt gyro samples. **Strength** (default 50%) sets how aggressive the detection is; too high may also flatten real fast moves. |
| **Rotation** | Rotates the motion data relative to the camera, in **Pitch**, **Roll** and **Yaw**. For blackbox data, set **Pitch** to the camera's uptilt angle. Right-click the checkbox for **Separate accelerometer rotation**, which adds an **Accelerometer rotation** with its own angles. |
| **Gyro bias** | Subtracts a constant rotation rate per axis (°/s). Filled in by **Estimate gyro bias here** in the timeline menu. |
| **IMU orientation** | How the gyro axes map to the camera axes, as three letters: uppercase is positive, lowercase negative (for example `zYX`). Usually detected; **Guess IMU orientation here** in the timeline menu can find it. |
| **Integration method** | How gyro (and accelerometer) samples become camera orientations: **Complementary**, **VQF** (the default for raw gyro data), **Simple gyro**, **Simple gyro + accel**, **Mahony** or **Madgwick**. For files with built-in orientation data there is also **None**, which uses that data as it is. |
| **Orientation indicator** | Draws the camera's orientation at the playhead. |
| **Statistics** | Shows frequency charts for each gyro and accelerometer axis, which help to pick a **Low pass filter** cutoff. |
| **Export** | Writes the motion data out: **Export camera data (CSV/JSON/USD/AE)**, **Export full metadata**, **Export parsed metadata** and **Export project file (including processed gyro data)**. |

For DJI O4 Pro and O4 Lite clips the panel also shows the Gyrogate section: what FPVFlow found in the
clip, **Remove DJI Gyrogate false rotations**, its **Sensitivity**, and **Save corrected copy**.
See [DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}).

{: .tip }
If the stabilized image turns the wrong way, or one axis seems swapped, the IMU orientation is
wrong. If the horizon drifts with horizon lock on, try another integration method.

## External audio

An FPVFlow panel: adds a separate audio recording to the clip, syncs it and mixes it into the
export. See [External audio]({{ '/external-audio.html' | relative_url }}).

## Synchronization

Aligns the motion data with the video: **Auto sync**, **Rough gyro offset**, **Sync search size**,
**Max sync points** and the analysis options under **Advanced**. Every control is explained in
[Synchronization]({{ '/guide/synchronization.html' | relative_url }}#the-synchronization-panel).

## Stabilization

### Smoothing method

The dropdown at the top chooses how the camera path is smoothed. The controls below it change with
the method.

| Method | What it does |
|---|---|
| **No smoothing** | Keeps the original motion. Only the lens correction (and horizon lock, if on) is applied. |
| **Default** | Adapts the smoothing to the motion: slow movements are smoothed strongly, fast ones less, so quick moves stay in frame. The usual choice for FPV. |
| **Plain 3D** | The same amount of smoothing for slow and fast motion, set as a time constant in seconds. |
| **Fixed camera** | Points the virtual camera in one fixed direction, set with **Roll angle**, **Pitch angle** and **Yaw angle**. |

Controls of **Default**:

| Control | What it does |
|---|---|
| **Smoothness** | Overall strength, in percent. |
| **Per axis** (under **Advanced**) | Replaces **Smoothness** with **Pitch smoothness**, **Yaw smoothness** and **Roll smoothness**. |
| **Only within trim range** (under **Advanced**) | Computes the smoothing only for the trimmed part, so motion outside it does not affect the result. On by default. |
| **Max smoothness** (under **Advanced**) | The smoothing time used for slow motion (default 1 s). |
| **Max smoothness at high velocity** (under **Advanced**) | The smoothing time used for fast motion (default 0.1 s). |

**Plain 3D** has **Smoothness** (in seconds) and **Only within trim range**.

### Lock horizon

Keeps the horizon level, whatever the drone does in roll. Some flips and rolls will look like the
world turning around a level camera.

| Control | What it does |
|---|---|
| **Lock amount** | 100% holds the horizon fully level, lower values let some of the roll through. Can be keyframed. |
| **Roll angle correction** | Tilts the locked horizon by a fixed angle, to fix a horizon that is level but slightly off. Shortcut keys: <kbd>E</kbd> and <kbd>R</kbd> (see [shortcuts]({{ '/guide/shortcuts.html' | relative_url }})). |
| **Lock pitch angle** / **Pitch angle correction** | Also holds the pitch at a fixed angle. |
| **Use gravity vectors** | Appears when the camera recorded its own gravity direction. Uses it for the horizon. |
| **Integration method** | Appears for clips that use built-in orientation data and also contain raw gyro data. Chooses how the horizon is tracked. |
| **Automatic lock** | Lets the horizon bank with the turns: when the roll rate goes above **Turn threshold**, the horizon tilts in proportion to it (**Turn multiplier**), smoothed by **Turn smoothness**. **Limit tilt acceleration** with **Tilt acceleration limit** keeps that tilt from changing abruptly. While it is on, the preview shows the current **Turn Speed (Roll):**. |

### Zoom and framing

The line under the horizon options shows the largest rotation the stabilization had to compensate
on each axis and the largest zoom it needed.

| Control | What it does |
|---|---|
| Zoom mode | **No zooming** (black borders can appear), **Dynamic zooming** (zooms in only as much as each moment needs) or **Static zoom** (one zoom for the whole clip, enough to hide all borders). |
| **Zoom limit** | The maximum zoom FPVFlow may use (default 130%). When the limit is reached, the smoothing is reduced instead, so the frame stays filled. Shown for the **Default** and **Plain 3D** methods. |
| **Zooming speed** | With **Dynamic zooming**, how quickly the zoom follows the need (default 4 s). Longer is calmer. |
| **Lens correction** | How much of the lens distortion is removed (default 100%). Lower values keep some of the fisheye look. |
| **Stabilize focal length** | Appears for clips with per-frame focal length data (zoom lenses). Smooths focal length changes. |

Under **Advanced**:

| Control | What it does |
|---|---|
| **FOV** | Scales the field of view of the output. Above 1.0 you may see borders; **Advanced** > **Safe area guide** helps there. |
| **Rolling shutter correction** | Corrects rolling shutter skew, given the **Frame readout time** in ms and the readout direction. See [Rolling shutter]({{ '/guide/calibration.html' | relative_url }}#rolling-shutter). |
| **Video speed** | Speeds up or slows down the output (10% to 1000%), and can be keyframed for speed ramps. The three small checkboxes next to it choose whether the speed also scales the smoothing, the zooming speed and the zoom limit. Audio export is not available when the speed is changed. |
| **Zooming method** | **Envelope follower** (default) or **Gaussian filter**, the curve used for dynamic zooming. Keep **Envelope follower** when keyframing the zooming speed. |
| **Additional 3D rotation** | Adds a fixed **Pitch**, **Yaw** and **Roll** to the output camera, for reframing. Can be keyframed. |
| **Zoom limit iterations** | Accuracy of the **Zoom limit** calculation (default 5). More is more accurate but slower. |

## Color

An FPVFlow panel: applies a `.cube` LUT and colour adjustments during the export. See
[Colour grading]({{ '/colour-grading.html' | relative_url }}).

## Export settings

| Control | What it does |
|---|---|
| Codec | **H.265/HEVC** (default), **H.264/AVC**, **ProRes**, **DNxHD**, **CineForm**, **EXR Sequence** or **PNG Sequence**. ProRes, DNxHD and PNG have a second dropdown for the variant. |
| **Output size** | Width and height, with an aspect ratio lock and a menu of size presets (**Original**, sizes for common aspect ratios, and one **Based on "Max zoom"**). |
| **Bitrate** | In Mbps, for H.264 and H.265. |
| **Use GPU encoding** | Much faster, but needs a higher bitrate for the same quality. Untick for maximum quality. |
| **Export audio** | Includes the audio. Also controls the external audio track. |

Under **Advanced**: **Custom encoder options**, **Metadata comment**, **Keyframe distance**,
**Preserve other tracks**, **Use black frames outside trim range and keep original file duration**,
**Export trim ranges as separate videos**, **Audio codec**, **Interpolation method**, **Device for
rendering**, **Preserve export settings** and **Preserve export path**. On some systems there are
also experimental hardware encoders.

What these do, and how the export and the render queue work, is described in
[Exporting and the render queue]({{ '/export.html' | relative_url }}).

## Advanced

Preview, background and application settings.

| Control | What it does |
|---|---|
| **Preview resolution** | Resolution of the preview player. Lower it if playback stutters. Does not affect the export. |
| **Background mode** | What fills areas outside the image when you zoom out: **Solid color**, **Repeat edge pixels**, **Mirror edge pixels** or **Margin with feather** (with **Margin** and **Feather**). |
| **Render background** | The colour for **Solid color**, as a hex value. |
| **Theme**, **UI scaling**, **Language** | Appearance of the app. |
| **Safe area guide** | When **FOV** is above 1, draws a rectangle showing what FOV 1 would include. Preview only. |
| **Use GPU decoding** | Decodes the video on the GPU. Turn it off if a file fails to decode. |
| **Format for R3D conversion** | Appears when REDline is installed. The ProRes flavour used to convert RED files before rendering. |
| **Preview pipeline** | How preview frames travel between decoder, GPU and screen. Change it only if the preview misbehaves. |
| **Device for video processing** | The GPU (or **CPU only**) used for the stabilization. |
| **Default file suffix** | Added to the output file name (default `_stabilized`). |
| **Notification sounds** | Plays a sound when a render finishes or fails. |
| **Reset all settings to default** | Clears all preferences after a confirmation. Lens profile favourites are kept. Restart the app afterwards. |
| **Open data folder** | Opens FPVFlow's data folder. Your own lens profiles and presets are in its `lens_profiles` folder. |

If a video editor with a supported Gyroflow plugin is detected, an extra **Video editor plugins**
panel appears below **Advanced**.
