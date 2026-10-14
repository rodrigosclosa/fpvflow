---
layout: default
title: Motion data logging
has_children: true
description: >-
  When an FPV camera needs an external gyro logger for FPVFlow, and which loggers work: Betaflight
  blackbox, the Gyroflow .gcsv format, phone apps and DIY loggers, plus how to set the IMU
  orientation.
nav_order: 8
---

# Motion data logging

FPVFlow stabilizes video with the rotation measured by a gyroscope, not by analysing the image. That
motion data has to come from somewhere: either the camera writes it into the clip, or a separate
device records it while you film.

## Do you need an external logger?

**Probably not, if your camera already records its own gyro.** DJI O3 and O4 air units, Avata,
GoPro HERO 5 and later, Insta360, many Sony bodies, RunCam Thumb and the other sources in the
[supported list](https://github.com/rodrigosclosa/fpvflow#supported-gyro-sources) store motion data
in or next to the video. FPVFlow loads it together with the clip and the **Motion data** panel shows
the **Detected format**.

{: .tip }
For DJI O4 footage, record with RockSteady and HorizonSteady off. FPVFlow needs the raw attitude
track to stabilize the clip and to repair the false rotations of the O4 gyro. See
[DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}).

**Yes, if your camera has no gyro metadata.** Typical cases on a quad are a "naked" cinema camera,
an older action camera, or any camera that is not on the supported list. Then you record the motion
with another device that is fixed rigidly to the camera, and load that file in **Motion data** with
**Open file**.

The most common logger for FPV pilots is the flight controller you already fly with: its blackbox
records the gyro at a high rate, and FPVFlow reads Betaflight logs directly.

## How an external log is used

1. Open the video in FPVFlow.
2. In **Motion data**, click **Open file** and choose the log. If the log has the same base name as
   the video and sits in the same folder, FPVFlow finds it by itself when you open the video. This
   works for MP4, MOV and MKV videos that carry no motion data of their own. It looks for `.jsonl`,
   `.gcsv`, `.bbl`, `.bfl` and `.csv` files, in that order.
3. Check the **IMU orientation** and, for a tilted camera, the **Rotation**.
4. Synchronize. An external log never starts at exactly the same moment as the video, so the time
   offset between the two has to be found with **Auto sync** in **Synchronization**.

The [Getting started]({{ '/getting-started.html' | relative_url }}) page walks through the whole workflow.

## Pages in this section

| Page | What it covers |
|:-----|:---------------|
| [Betaflight blackbox]({{ '/logging/betaflight.html' | relative_url }}) | Using the flight controller (or a second FC on the camera) as the gyro logger: blackbox setup, loading the log, camera angle and sync. |
| [Other loggers]({{ '/logging/other-loggers.html' | relative_url }}) | Every other source FPVFlow reads that works as an external logger: ArduPilot, phone apps, ESP32 loggers, WitMotion sensors, cameras that write a separate gyro file. |
| [IMU orientation]({{ '/logging/orientation.html' | relative_url }}) | What the three-letter orientation string means and how to find the right one for your logger. |
| [The .gcsv format]({{ '/logging/gcsv.html' | relative_url }}) | The simple text format for your own logger: header, columns, units and an example. |

## Tips that apply to every logger

- **Mount it rigidly.** The logger must move exactly like the camera. Soft foam, a loose strap or a
  vibrating arm adds motion the camera never had.
- **Keep the axes simple.** Mount the logger square to the camera (edges parallel) so the
  orientation is a plain axis swap. A deliberate tilt is fine; you enter it as a **Rotation**.
- **Start the log before the video and stop it after.** FPVFlow can only stabilize the part of the
  clip that is covered by motion data.
- **Give sync something to see.** A few quick, sharp movements near the start of the recording
  make the offset between video and log easy to find.
