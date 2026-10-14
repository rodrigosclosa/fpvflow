---
layout: default
title: Exporting and the render queue
description: "Export a stabilized FPV clip with FPVFlow, batch-render with the render queue, and know what each queued job keeps: external audio, LUT, colour adjustments and the Gyrogate setting."
nav_order: 6
---

# Exporting and the render queue
{: .no_toc }

How to get the stabilized, graded clip out of FPVFlow, one at a time or in a batch.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## Export a single clip

1. Set the file name and folder in **Output path:** in the bottom bar.
2. Check the **Export settings** panel (see below).
3. Click **Export**, or press <kbd>Ctrl</kbd>+<kbd>W</kbd> (<kbd>Cmd</kbd>+<kbd>W</kbd> on macOS).

When the render finishes, FPVFlow offers **Open rendered file** and **Open file location**.

Before it starts, FPVFlow stops you in a few cases:

- **Lens profile is not loaded** - the result will be wrong. Load a profile, or answer **Yes** to
  export anyway.
- **There are no sync points present** - the motion data was never synchronized. This check is
  skipped for clips whose motion data is already synced to the video, such as DJI clips.
- **Output file already exists** - choose **Yes** to overwrite, **Rename** to add a number, or
  **No** to cancel.

{: .note }
If another render is already running and no parallel slot is free, **Export** adds the clip to the
render queue instead of starting right away.

## Export settings essentials

The **Export settings** panel has the options you change most often at the top:

| Setting | What it does |
|---|---|
| Codec | **H.265/HEVC** (the default), **H.264/AVC**, **ProRes**, **DNxHD**, **CineForm**, or a **PNG Sequence** / **EXR Sequence**. ProRes, DNxHD and CineForm produce a `.mov`; H.264 and H.265 a `.mp4`. |
| **Output size** | Output width and height. Keep the aspect ratio locked unless you are reframing. |
| **Bitrate** | For H.264 and H.265. GPU encoders need a higher bitrate for the same quality. |
| **Use GPU encoding** | Much faster, slightly lower quality per bit. Turn it off for maximum quality. |
| **Export audio** | Embeds the audio in the output. Also the switch for the external audio track. |

Under **Advanced** you find **Audio codec**, **Interpolation method**, **Custom encoder options**,
**Export trim ranges as separate videos**, **Preserve export settings** and **Preserve export
path** (reuse the size, bitrate and folder for every clip), among others.

These settings come from Gyroflow unchanged. For codec choice, hardware encoding and every
advanced option, see [Exporting](https://docs.gyroflow.xyz/app/advanced-usage/exporting) in the
Gyroflow docs.

### Audio in the export

- With an external audio track loaded and **Export audio** on, the external track replaces the
  camera's audio in the output, at the offset you set.
- The **External audio** panel tells you whether the track's format is kept as is for the current
  output, or converted. A 32-bit float WAV, for example, does not fit in a `.mp4`: the panel then
  suggests picking ProRes, DNxHD or CineForm to get a `.mov`, otherwise the audio is converted.
- PNG and EXR sequences have no audio, and audio export is not available when you change the
  video speed.

More on the track itself on the [External audio page]({{ '/external-audio.html' | relative_url }}).

### LUT and colour in the export

The LUT and the colour adjustments from the **Color** panel are applied while the stabilized
frames are rendered, so the exported file is already graded. When a LUT is loaded, the output is
tagged Rec.709, so players show it correctly instead of treating it as log footage.

Grading is not free, but it is fast: the colour conversion uses all CPU cores and a light chroma
filter. A 4K export with a LUT went from about 7 to about 30 fps on a Mac with an M4 Pro.

{: .tip }
If you need the slower, slightly more precise conversion, turn on **Precise color conversion
(slower export)** in the **Color** panel. It costs a lot of render speed and the difference is
rarely visible. The choice is stored with each queued job.

See [Colour grading]({{ '/colour-grading.html' | relative_url }}) for the LUT library and the
adjustments.

## The render queue

The render queue lets you set up several clips and render them in one go, for example after a
day of flying.

### Add clips to the queue

1. Set up the clip as usual: lens, motion data, sync, stabilization, colour, audio.
2. Click the small arrow on the export button and choose **Add to render queue**. This adds the
   current clip right away and switches the button to queue mode, so the next clicks also add to
   the queue. Alternatively, press <kbd>Ctrl</kbd>+<kbd>Q</kbd> to add the current clip in either
   mode.
3. Open the next clip and repeat.
4. Open the queue with the **Render queue** button next to the export button (or press
   <kbd>Q</kbd>) and click **Start exporting**.

{: .warning }
To leave queue mode, choose **Export** in the same menu. That also starts exporting the clip
that is open right away, so switch back only when you want that clip exported now.

While the queue runs you can **Pause** and **Resume** it, and see the elapsed and remaining time
of each job. Right-click a job for **Render now**, **Edit**, **Move up**, **Move down** and
**Reset status**.

The **Queue settings** menu sets the **Number of parallel renders**, the **Default overwrite
action** (**Ask**, **Overwrite file**, **Rename file**, **Skip file**), the **Export mode**
(stabilized video, project file, or both) and what to do **When rendering is finished**. The
Gyroflow docs describe these in [Render queue](https://docs.gyroflow.xyz/app/advanced-usage/render-queue).

{: .note }
The queue survives closing the app. When you open FPVFlow again with unfinished jobs, it says **You
have unfinished tasks in the render queue** and offers **Open render queue**.

### What each queued job keeps

A job is a snapshot of the clip as it was when you added it. Besides the Gyroflow settings (lens
profile, motion data, sync points, stabilization and export settings), FPVFlow keeps its own
per-clip settings with every job:

| Setting | Kept per job |
|---|---|
| External audio | The audio file, its sync offset and the **Preserve original audio format** choice |
| Colour | The LUT, its **Intensity**, the colour adjustments and **Precise color conversion (slower export)** |
| Gyrogate | Whether **Remove DJI Gyrogate false rotations** is on, and its **Sensitivity** |

So you can queue clips with different LUTs, different audio tracks, and DJI O4 clips with and
without the Gyrogate repair, and each one renders with its own settings. These settings are also
restored, per job, when the queue is reloaded after reopening the app.

{: .warning }
The per-clip settings are taken from the clip as you had it open in the main window. A video that
goes into the queue without being opened there first (for example several files dropped at once
with **Add to render queue**) is not set up as a clip: it gets no LUT and no Gyrogate repair, and
its audio settings are not its own. Open each such job with **Edit**, check the **Color**,
**External audio** and **Motion data** panels, and click **Save**.

### Edit a queued job

1. In the render queue, right-click the job and choose **Edit**. (Not available while that job is
   rendering.)
2. The clip opens in the main window with all its settings, including its external audio, LUT
   and Gyrogate setting. The export button now reads **Save**.
3. Change what you need.
4. Click **Save** (or press <kbd>Ctrl</kbd>+<kbd>W</kbd> or <kbd>Ctrl</kbd>+<kbd>Q</kbd>). The job is
   updated in place in the queue; nothing is rendered until the queue runs.

{: .important }
Opening a different video while you edit a job ends the edit. The button goes back to **Export**
or **Add to render queue**, and the job in the queue keeps its previous settings.

## Project files and editor plugins

Instead of rendering in FPVFlow, you can save the stabilization as a `.gyroflow` project: use
**Export project file** or **Export project file (including gyro data)** from the export button's
menu, or press <kbd>Ctrl</kbd>+<kbd>S</kbd>. FPVFlow keeps the external audio and colour settings
in the project, so reopening it in FPVFlow restores them.

Projects can also be loaded by the Gyroflow video editor plugins (DaVinci Resolve, Premiere,
After Effects, Final Cut Pro) to stabilize without transcoding. See the
[general plugin workflow](https://docs.gyroflow.xyz/app/video-editor-plugins/general-plugin-workflow).

{: .note }
The LUT, the colour adjustments and the external audio track are FPVFlow features applied in
FPVFlow's own render. In a plugin workflow, grade and add the audio in your editor. For DJI O4
clips, use **Save corrected copy** in the **Motion data** panel so the plugin reads motion data
without the Gyrogate false rotations.
