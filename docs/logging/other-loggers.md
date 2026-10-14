---
layout: default
title: Other loggers
parent: Motion data logging
nav_order: 2
description: >-
  External gyro sources FPVFlow can read besides Betaflight: ArduPilot logs, phone sensor apps,
  ESP32 and WitMotion loggers, Flowshutter, RunCam-style gyro CSV files and a second camera.
---

# Other loggers
{: .no_toc }

Besides the [Betaflight blackbox]({{ '/logging/betaflight.html' | relative_url }}), FPVFlow reads motion data
from a range of other devices. It reads them through telemetry-parser, the same library Gyroflow
uses, at a newer revision. Whatever the source, the workflow
is the same: mount the logger rigidly on the camera, load its file with **Open file** in
**Motion data**, check the [IMU orientation]({{ '/logging/orientation.html' | relative_url }}) and synchronize.

1. TOC
{:toc}

---

## A second camera with a gyro

Any camera FPVFlow supports can act as the logger for one that is not supported: mount a small
GoPro, a DJI Action or an Insta360 rigidly on the main camera, record with both, and load the
action camera's clip as the motion data of the main video.

- Leave **Load all metadata** unchecked. With it checked, FPVFlow also takes the lens profile,
  frame rate and other clip settings from that file, which belong to the action camera, not to
  your main camera.
- **Frame offset** shifts the video by whole frames against the motion data, if you need it.

## Flight controller firmware

| Source | Files | Notes |
|:-------|:------|:------|
| Betaflight and compatible blackbox | `.bbl`, `.bfl`, `.csv` | See [Betaflight blackbox]({{ '/logging/betaflight.html' | relative_url }}). |
| ArduPilot | `.bin`, `.log` | Binary DataFlash logs, or text logs that contain `VSTB` messages. In `.bin` logs FPVFlow uses the `VSTB` (video stabilization) messages when present, then `IMU`, then `GYR`/`ACC`. |

## Flowshutter

[Flowshutter](https://github.com/gyroflow/flowshutter) is an open-source camera remote from the
Gyroflow community. It starts and stops the camera and a Betaflight-based logger together, so the
motion log and the video begin at almost the same time. The motion data it produces is a
regular Betaflight blackbox log: load and set it up as described on the
[Betaflight blackbox]({{ '/logging/betaflight.html' | relative_url }}) page. Hardware, wiring and supported
cameras are documented in the Flowshutter repository.

## DIY and dedicated loggers

| Logger | Files | Notes |
|:-------|:------|:------|
| [esp-gyrologger](https://github.com/VladimirP1/esp-gyrologger) | `.bin` | Small ESP32-based logger. FPVFlow recognises its log by the file header. |
| WitMotion IMU modules (such as the WT901SDCL) | `.bin`, `.txt` | The binary log and several of the text export layouts. |
| SenseFlow | `.bin` | Binary logs. |
| Your own logger | `.gcsv` | Write the documented [.gcsv format]({{ '/logging/gcsv.html' | relative_url }}): you declare the scales, the time unit and the orientation in the header. |

## Phone apps

A phone strapped to the camera works as a logger for slow, smooth shots. FPVFlow recognises the
logs of these apps by their content:

- **Sensor Logger** (iOS and Android)
- **G-Field Recorder** (iOS)
- **Gyro** (iOS)
- **Sensor Record** (Android)
- **OpenCamera Sensors** (Android)
- **Film it** (motion track stored inside the MP4)

{: .note }
Phone sensors are less precise and often sample slower than a flight controller or an action
camera, and a phone is heavy to mount rigidly. For flying footage, prefer the blackbox or an action
camera.

## Cameras that write a separate gyro file

Some FPV and action cameras record their gyro in a CSV file next to the video instead of inside it.
Keep the original file names and both files in the same folder: for the cameras below that have a
naming pattern, FPVFlow finds the CSV by itself when you open the MP4.

| Camera | Video | Gyro file FPVFlow looks for |
|:-------|:------|:----------------------------|
| RunCam 5 Orange | `RC_<number>_<...>.MP4` (original name) | `RC_GyroData<number>.csv` |
| iFlight GOCam GR | `IF-RC<...>_<number>.MP4` | `gyroData<number>.csv` or `gyroDate<number>.csv` |
| RunCam Thumb | `Thumb<...>.mp4` | same name with `.csv` |
| Mobius Maxi 4K | any | no naming pattern: give the `.csv` the same name as the video, or load it with **Open file**; it is recognised by its column header |

Each model gets its own preset for scale and orientation.

{: .warning }
A generic CSV whose header starts with `time,x,y,z` is read with the same layout: time in
milliseconds and raw 16-bit gyro values with a fixed range. That is rarely what a custom logger
writes. For your own data, use the [.gcsv format]({{ '/logging/gcsv.html' | relative_url }}) instead, where you
set the scales yourself.

## Gyroflow protobuf and JSONL

Newer devices and tools can write motion data in the Gyroflow protobuf telemetry format, either
embedded in an MP4 or MOV file or as a `.jsonl` file next to the video. FPVFlow reads both. When a
video has several possible log files next to it, the `.jsonl` file is tried first, because it can
also carry the lens and timing information of the clip.

## Not on this page?

The full list of cameras and devices, including the ones that record their own gyro, is in the
[Supported gyro sources](https://github.com/rodrigosclosa/fpvflow#supported-gyro-sources) section of
the README. If your logger is not there, converting its output to [.gcsv]({{ '/logging/gcsv.html' | relative_url }})
is usually the quickest way to use it.
