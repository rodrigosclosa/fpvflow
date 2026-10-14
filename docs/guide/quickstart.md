---
layout: default
title: Quick start
parent: Guide
nav_order: 1
description: "Stabilize an FPV clip with FPVFlow in seven steps: open the video, check the lens profile and motion data, synchronize, choose the smoothing and export."
---

# Quick start

The short version, for when you already know what gyro stabilization is. For a full walkthrough of
one clip, with the questions FPVFlow may ask along the way, read
[Getting started]({{ '/getting-started.html' | relative_url }}).

1. **Get FPVFlow.** Download it from the
   [releases page](https://github.com/rodrigosclosa/fpvflow/releases/latest) and follow
   [Installation]({{ '/installation.html' | relative_url }}).
2. **Open the clip.** Drop the video on the preview (it reads **Drop video file here**) or click
   **Open file** in **Video information**. Camera, lens and motion data are detected while the
   file loads.
3. **Check the lens profile.** If **Lens profile** shows a profile that matches your camera and
   recording mode, you are done here. If not, type the camera name in the search box and pick the
   right entry. No profile for your camera? See
   [Lens profiles and calibration]({{ '/guide/calibration.html' | relative_url }}).
4. **Check the motion data.** Clips from cameras that log their own motion (DJI air units, GoPro,
   Insta360 and others) carry it inside the video. For a separate log, such as a Betaflight
   blackbox file, click **Open file** in **Motion data** and select it.
5. **Synchronize, if needed.** Clips whose motion data is already in sync with the video (DJI, for
   example) need nothing, and FPVFlow folds the **Synchronization** panel away. For anything else,
   click **Auto sync** and check the result. Details in
   [Synchronization]({{ '/guide/synchronization.html' | relative_url }}).
6. **Choose the look.** Pick a method and a **Smoothness** value in **Stabilization**, decide
   whether to **Lock horizon**, and play the clip. Press <kbd>S</kbd> to compare with the
   unstabilized image.
7. **Export.** Set the file name in **Output path:** at the bottom and click **Export**. See
   [Exporting and the render queue]({{ '/export.html' | relative_url }}).

{: .tip }
Turn off the camera's own stabilization when you fly (RockSteady, HorizonSteady, HyperSmooth and
similar). Gyro stabilization needs the raw image. On DJI O4 units it also keeps the 1 kHz attitude
data that the [Gyrogate repair]({{ '/gyrogate.html' | relative_url }}) depends on.

Optional extras that FPVFlow adds at this point: a `.cube` LUT and colour adjustments in the
**Color** panel ([Colour grading]({{ '/colour-grading.html' | relative_url }})), and a separate audio recording
in **External audio** ([External audio]({{ '/external-audio.html' | relative_url }})).
