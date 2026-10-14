---
layout: default
title: Other cameras
parent: Cameras
nav_order: 8
description: >-
  Other motion data sources FPVFlow reads: Canon, XTRA, Freefly Ember, XTU, Hawkeye Firefly X
  Lite, Vuze, KanDao, WitMotion, CAMM files, and what to do if your camera is not listed.
---

# Other cameras
{: .no_toc }

Besides the brands with their own page, FPVFlow reads motion data from the sources below. They
all come from the Gyroflow 1.6.3 list, since FPVFlow uses the same reader.

1. TOC
{:toc}

---

## Cameras with motion data in the file

| Camera | Notes |
|:-------|:------|
| Canon C50, C80, C400, EOS R6 Mark III, EOS R5 Mark II | `.mp4`, `.mov` and `.mxf`. Aligned with the frames, no sync needed. |
| XTRA Edge, Edge Pro | Aligned with the frames, no sync needed. |
| Freefly Ember | Aligned with the frames, no sync needed. |
| Vuze XR | |
| KanDao QooCam EGO | `.mp4`, `.mov`. |
| Files in the CAMM format | [CAMM](https://developers.google.com/streetview/publish/camm-spec) is an open metadata format for camera motion. Any camera that writes it can be used. |

For these cameras, open the clip and check **Contains gyro** in **Video information**. If the
**Synchronization** panel opens collapsed, the data is already aligned; otherwise, load a lens
profile and run **Auto sync**.

## Cameras with a separate log

| Camera or device | Notes |
|:-----------------|:------|
| Hawkeye Firefly X Lite | CSV log. |
| KanDao Obsidian Pro | The motion data is in a separate `imu.bin` file, not in the video. |
| WitMotion WT901SDCL | A stand-alone IMU that logs to `.bin` or `.txt`. Mount it rigidly on the camera. |

Load the log with **Open file** in **Motion data**, then sync with **Auto sync**.

## Other cameras in the Gyroflow list

The Gyroflow 1.6.3 list also names the XTU S2 Pro and S3 Pro. We could not confirm from the
code where these cameras keep their motion data, so open a test clip first and check
**Contains gyro** in **Video information**. If it says no, look for a log file next to the video
and load it in **Motion data**.

## FPV drones and external loggers

For a camera without a gyro on an FPV drone, the flight controller is the logger:

- Betaflight blackbox logs (`.bbl`, `.bfl`, and `.csv` exported from Blackbox Explorer).
  If a log holds several flights, pick the right one in **Select log**.
- ArduPilot logs (`.bin`, `.log`).

Set the camera angle under **Rotation** in **Motion data** (**Pitch** is the camera uptilt) and
check **IMU orientation** if the motion goes the wrong way. The camera must be mounted rigidly to
the frame, because the flight controller measures the frame, not the camera.
[Getting started]({{ '/getting-started.html' | relative_url }}) covers these steps.

Dedicated loggers such as [esp-gyrologger](https://github.com/VladimirP1/esp-gyrologger) and
[flowshutter](https://github.com/gyroflow/flowshutter), and any device that writes Gyroflow's
`.gcsv` format, work the same way.

## My camera is not listed

- **It has a built-in gyro.** Support for new cameras is added to Gyroflow's motion data reader
  ([telemetry-parser](https://github.com/AdrianEddy/telemetry-parser)). Ask in the
  [Gyroflow](https://github.com/gyroflow/gyroflow) project, ideally with a short sample clip;
  it reaches FPVFlow when FPVFlow updates to a newer version of that library.
- **It has no gyro.** Record the motion with another device fixed to the camera: a flight
  controller, a gyro logger, an action camera that records motion data, or a phone running a
  logging app (see [Smartphones]({{ '/cameras/smartphones.html' | relative_url }})). Then load that log in
  **Motion data** and sync it.
