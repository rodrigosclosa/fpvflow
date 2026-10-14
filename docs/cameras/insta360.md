---
layout: default
title: Insta360
parent: Cameras
nav_order: 3
description: >-
  Stabilize Insta360 GO, ONE R, ONE RS, SMO 4K, Ace and Caddx Peanut footage in FPVFlow: built-in
  motion data, lens profile from the file and recording settings.
---

# Insta360

Insta360 cameras store gyro and accelerometer samples in the video file, along with metadata
about the shot: the field of view mode, whether in-camera stabilization was on and, on many
models, the lens calibration and the orientation of the motion sensor. FPVFlow reads all of it
when you open the clip.

## Supported models

Following the Gyroflow 1.6.3 list: ONE R, ONE RS, SMO 4K, GO, GO 2, GO 3, GO 3S, GO Ultra, Ace,
Ace Pro, and the Caddx Peanut. Both `.mp4` and `.insv` files open directly.

The GO series and the Peanut are popular on small FPV quads, where a full-size action camera is
too heavy.

## What the file gives you

- **Motion data aligned with the frames**, so no synchronization is needed. FPVFlow collapses
  the **Synchronization** panel for these clips.
- **Lens information.** FPVFlow reads the field of view mode and whether in-camera stabilization
  was on, and uses it to choose the lens profile. For several models the lens calibration stored
  in the file is used directly.

Check **Detected lens** in **Video information** and the profile in **Lens profile** after
opening a clip.

## Recording settings

- **Widest field of view.** Pick the mode that records the most of the sensor. More image means
  more room for the crop.
- **In-camera stabilization off** where the mode allows it, so the image reaches FPVFlow
  unwarped.
- **Soft-mount on a drone.** The gyro is inside the camera, so keep motor vibration away from it.

See [Cameras]({{ '/cameras/' | relative_url }}) for shutter speed and general advice.
