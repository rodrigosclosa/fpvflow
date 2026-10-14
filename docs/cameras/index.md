---
layout: default
title: Cameras
has_children: true
description: >-
  Which cameras work with FPVFlow and Gyroflow-style stabilization: built-in gyro versus an
  external logger, how to check that a clip carries motion data, and the recording settings that
  give the best result.
nav_order: 7
---

# Cameras
{: .no_toc }

FPVFlow stabilizes video with the motion data recorded while you filmed. A camera is "supported"
when FPVFlow can read that motion data and knows how the lens bends the image. This section
explains what that means in practice and lists the settings to use for each brand.

1. TOC
{:toc}

---

## What makes a camera supported

Three things decide how well a clip stabilizes:

- **Motion data.** A gyroscope (and usually an accelerometer) sampled fast enough to follow the
  camera's movement. It can live inside the video file, in a file the camera writes next to it,
  or in a separate logger.
- **Timing.** The motion data has to line up with the frames. Some cameras record their
  orientation already aligned with each frame, so there is nothing to synchronize. Others record
  raw gyro samples on their own clock, and FPVFlow has to find the offset with **Auto sync**.
- **A lens profile.** It describes the distortion of the lens at the resolution you recorded.
  Many cameras write it into the file, or FPVFlow finds it in the built-in database; otherwise you
  can search for one or calibrate your own.

FPVFlow reads motion data with the same library as Gyroflow 1.6.3, so every camera Gyroflow
supports works the same way here.

## Check what a clip contains

After opening a clip:

1. In **Video information**, look at **Detected camera**, **Detected lens** and **Contains gyro**.
   If **Contains gyro** says no, the file carries no motion data that FPVFlow can read.
2. In **Motion data**, **Detected format** shows which reader picked up the data.
3. In **Lens profile**, check that a profile is loaded and that no warning about mismatched
   dimensions or aspect ratio is shown.

If the camera recorded frame-aligned orientation in the video file, FPVFlow collapses the
**Synchronization** panel: there is nothing to sync, and extra sync points only make the result
worse. Otherwise, run **Auto sync** after the lens profile is loaded.
The [Getting started]({{ '/getting-started.html' | relative_url }}) guide walks through these steps.

## Built-in gyro or external logger

**Cameras with a built-in gyro** are the easiest: the data is recorded with the video and loaded
automatically. Most action cameras, DJI FPV units and several mirrorless and cinema cameras fall
in this group. See the brand pages below.

**Cameras without a gyro** can still be stabilized with motion data from another device mounted
on the camera or on the aircraft:

- a Betaflight blackbox log from the flight controller (`.bbl`, `.bfl`, `.csv`);
- an ArduPilot log (`.bin`, `.log`);
- a dedicated gyro logger such as [esp-gyrologger](https://github.com/VladimirP1/esp-gyrologger)
  or [flowshutter](https://github.com/gyroflow/flowshutter), or any device that writes Gyroflow's
  `.gcsv` format;
- a smartphone running a sensor logging app (see [Smartphones]({{ '/cameras/smartphones.html' | relative_url }})).

Load the log with **Open file** in the **Motion data** panel, set the camera angle under
**Rotation**, and synchronize. An external log always needs **Auto sync**, because the logger
and the camera do not share a clock.

{: .note }
The log only describes the motion of the part it is attached to. With a flight controller log,
FPVFlow assumes the camera moved exactly like the frame, so anything that lets the camera move on
its own, like a loose or flexible mount, adds motion that the log does not contain. With a gyro in
the camera, the log follows the camera wherever it moves, and the concern becomes how much motor
vibration reaches it.

## Recording settings

These apply to almost every camera. The brand pages add the specifics.

- **Turn the in-camera stabilization off.** RockSteady, HorizonSteady, HyperSmooth, FlowState,
  electronic image stabilization: they crop and warp the image before FPVFlow sees it. Lens or
  sensor-shift stabilization moves the image independently of the gyro, which shows up as
  residual wobble.
- **Record the widest field of view the camera offers.** Stabilization crops into the frame, so
  every extra pixel at the edges is margin you can use. A 4:3 or full-sensor mode usually gives
  the most; crop to 16:9 at export.
- **Leave distortion correction off.** Keep the image as the lens produced it, so it matches the
  lens profile.
- **Choose the shutter speed for motion blur.** Stabilization removes the shake but not the blur
  that the shake left in each frame. A faster shutter keeps frames sharp after stabilization; a
  slower one looks more natural on smooth moves but can smear on sharp jolts. A common starting
  point is an exposure of about half the frame interval, the "180 degree" rule; in bright light
  that usually needs an ND filter.
- **Keep vibration away from the gyro.** Prop wash and motor noise in the motion data turn into
  jitter in the stabilized video. Balanced props, a well-tuned flight controller and a soft mount
  for cameras with their own gyro all help. The **Low pass filter** in **Motion data** can tame
  some leftover noise.

## Camera pages

| Page | Covers |
|:-----|:-------|
| [DJI]({{ '/cameras/dji.html' | relative_url }}) | O3 and O4 air units, Avata, Action series, Neo |
| [GoPro]({{ '/cameras/gopro.html' | relative_url }}) | HERO5 and later |
| [Insta360]({{ '/cameras/insta360.html' | relative_url }}) | GO, ONE R/RS, SMO 4K, Ace, Caddx Peanut |
| [RunCam]({{ '/cameras/runcam.html' | relative_url }}) | RunCam 5 Orange, Thumb, iFlight GOCam GR, Mobius Maxi 4K |
| [Sony]({{ '/cameras/sony.html' | relative_url }}) | Mirrorless, cinema and compact models with gyro metadata |
| [Smartphones]({{ '/cameras/smartphones.html' | relative_url }}) | Sensor logging apps on iOS and Android |
| [Cinema cameras]({{ '/cameras/cinema.html' | relative_url }}) | Blackmagic RAW and RED R3D |
| [Other cameras]({{ '/cameras/others.html' | relative_url }}) | Canon, XTRA, Freefly and other supported sources |

## FPVFlow additions

- DJI O4 footage can be affected by false rotations in its motion data. The
  [Gyrogate repair]({{ '/gyrogate.html' | relative_url }}) detects and removes them.
- Log footage (DJI D-Log M, GoPro Flat and similar) can be graded with a `.cube` LUT before
  export. See [Colour grading]({{ '/colour-grading.html' | relative_url }}).
- If you record sound on a separate microphone, [External audio]({{ '/external-audio.html' | relative_url }})
  adds and syncs that track.
