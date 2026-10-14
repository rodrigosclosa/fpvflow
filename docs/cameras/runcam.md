---
layout: default
title: RunCam
parent: Cameras
nav_order: 4
description: >-
  Stabilize RunCam 5 Orange, RunCam Thumb, iFlight GOCam GR and Mobius Maxi 4K footage in FPVFlow:
  gyro CSV logs next to the video, recording settings, soft mounting and synchronization.
---

# RunCam

A few small FPV cameras log their gyro to a CSV file written next to each video, instead of
inside it. They share one log format, which FPVFlow reads.

## Supported models

| Camera | Log file next to the video | Found automatically |
|:-------|:---------------------------|:--------------------|
| RunCam 5 Orange | `RC_GyroData<number>.csv` for `RC_<number>_<date>.MP4` | Yes |
| iFlight GOCam GR | `gyroData<number>.csv` (or `gyroDate...`) for `IF-RC_<number>.MP4` | Yes |
| RunCam Thumb | `.csv` with the same name as the `Thumb....mp4` | Yes |
| Mobius Maxi 4K | CSV log with a temperature column | Load it by hand |

These cameras only write the log with a firmware that supports gyro logging. Check the
manufacturer's download page for your model.

## Load the motion data

1. Copy the video and its CSV log to the same folder, without renaming either one.
2. Open the video. When the log is found automatically, the gyro chart appears in the timeline
   under the preview.
3. If it is not found (renamed files, or the Mobius), click **Open file** in **Motion data** and
   choose the CSV.

## Lens profile and sync

These logs are raw gyro samples on the camera's own clock, so every clip needs synchronization:

1. Pick the lens profile that matches the camera and the recording mode in **Lens profile**.
   Profiles for these cameras are in the built-in database; search for the model name.
2. Run **Auto sync** in **Synchronization**.
3. Play the result. If it jitters, try the **Low pass filter** in **Motion data**.

## Recording settings

- **Electronic stabilization off and distortion correction off.** The image must reach FPVFlow
  as the lens produced it.
- **The widest mode.** A native 4:3 mode keeps the full sensor. A 16:9 mode crops the sensor and
  leaves less margin for stabilization. If your camera stretches a 4:3 sensor into a 16:9 file,
  use the lens profile made for that mode.
- **Shutter speed.** See the general advice in [Cameras]({{ '/cameras/' | relative_url }}).

{: .warning }
Whatever shakes the camera body ends up in this gyro log, and then as jitter in the stabilized
result. On a drone, read the vibration advice in [Cameras]({{ '/cameras/' | relative_url }}) before you
choose how to mount these cameras.
