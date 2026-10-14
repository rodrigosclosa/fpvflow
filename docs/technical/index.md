---
layout: default
title: Technical details
has_children: true
description: >-
  How FPVFlow stabilizes FPV footage under the hood: the smoothing algorithms and horizon lock,
  rolling shutter correction, and the gyro filters including the DJI O4 Gyrogate repair.
nav_order: 9
---

# Technical details

These pages explain what happens between the motion data in your clip and the stabilized frame,
and what each setting in the **Stabilization** and **Motion data** panels actually changes. You
do not need them to get a good result with the defaults, but they help when a clip misbehaves:
the horizon wanders, fast rolls look wobbly, or a jolt shows up that was never in the raw
footage.

FPVFlow uses the Gyroflow stabilization engine: Gyroflow 1.6.3 plus later changes from Gyroflow's
development code, such as **Glitch filtering**, **Automatic lock** and **Lock pitch angle** for
horizon lock. Most of what is described here also applies to Gyroflow itself, except where a page
says a feature is FPVFlow only.

{: .note }
**Glitch filtering**, **Automatic lock** and **Lock pitch angle** are not in Gyroflow 1.6.3. To
use them in Gyroflow or in the video editor plugins you need a newer Gyroflow build.

| Page | What it covers |
|:-----|:---------------|
| [Smoothing algorithms]({{ '/technical/smoothing.html' | relative_url }}) | The four smoothing methods, their parameters, horizon lock, zooming and field of view. |
| [Rolling shutter correction]({{ '/technical/rolling-shutter.html' | relative_url }}) | Frame readout time and direction, where the value comes from and how the correction works. |
| [Gyro filtering]({{ '/technical/filtering.html' | relative_url }}) | Low pass, median and glitch filters, and the DJI O4 Gyrogate repair. |

## How the pieces fit together

1. **Motion data.** FPVFlow reads the camera orientation over time, either straight from the
   orientation the camera recorded or by integrating raw gyro and accelerometer samples. The
   filters described in [Gyro filtering]({{ '/technical/filtering.html' | relative_url }}) clean this data
   before anything else uses it.
2. **Synchronization.** The motion data is lined up with the video frames.
3. **Smoothing.** A smoothing algorithm turns the real, shaky camera path into a calm virtual
   camera path. Horizon lock, if enabled, is applied here too.
4. **Frame correction.** For every frame, the difference between the real and the virtual
   camera becomes a rotation that is applied to the image, together with lens undistortion and,
   if enabled, a per-row rolling shutter correction.
5. **Zooming.** The rotated frame no longer covers the whole output, so the image is cropped
   (zoomed) to hide the empty borders.
