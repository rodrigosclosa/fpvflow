---
layout: default
title: Sony
parent: Cameras
nav_order: 5
description: >-
  Stabilize Sony a1, a7 IV, a7S III, FX3, FX6, ZV-E10 and other gyro-equipped Sony cameras in
  FPVFlow: built-in motion data, lens metadata, IBIS and OSS, and recording settings.
---

# Sony

Many recent Sony cameras write a real-time metadata track into the video. Besides the gyro and
accelerometer readings, it can hold the focal length, lens distortion tables and the movement of
the stabilized sensor or lens. FPVFlow reads all of it, and any lens works as long as FPVFlow has a
profile for it, either from that metadata or from the lens database.

## Supported models

Following the Gyroflow 1.6.3 list: a1, a7C, a7R V, a7 IV, a7S III, a9 II, a9 III, a6700, FX3,
FX6, FX9, RX0 II, RX100 VII, ZV-1, ZV-E10, ZV-E10 II and ZV-E1. Files can be `.mp4`, `.mov` or
`.mxf`.

Not every recording mode necessarily writes motion data. After a test recording in the mode you
plan to use, check **Contains gyro** in **Video information**.

## What the file gives you

- **Gyro data aligned with the video**, so no synchronization is needed. FPVFlow collapses the
  **Synchronization** panel for these clips.
- **Lens metadata.** With lenses that report it, the camera writes the focal length and distortion
  data, and FPVFlow builds the lens profile from it. This also follows zoom changes during a shot.
- **Stabilization data.** When the file records the movement of the sensor (IBIS) or of the lens
  stabilization (OSS), FPVFlow takes it into account.

## Lenses without metadata

Manual lenses, adapted lenses and some third-party lenses report nothing to the camera. For
those, search for your lens in **Lens profile**, or calibrate one with **Create new**. The
profile must match the lens, the focal length and the recording resolution.

## Recording settings

- **In-body stabilization off.** Even with stabilization data in the file, the simplest and most
  predictable setup is SteadyShot off.
- **Active and digital stabilization modes off.** They crop and warp the image in camera.
- **Shutter speed.** See the general advice in [Cameras]({{ '/cameras/' | relative_url }}).
