---
layout: default
title: Rolling shutter correction
parent: Technical details
nav_order: 2
description: >-
  Fix jello and skew in FPV footage with FPVFlow's rolling shutter correction: what the frame
  readout time and direction are, where FPVFlow gets them, and how to measure or estimate them.
---

# Rolling shutter correction
{: .no_toc }

Most action and FPV cameras do not capture a frame all at once. They read the sensor one row at a
time, top to bottom, and that takes a few milliseconds. FPVFlow can use the gyro data to undo
the distortion this causes.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## What rolling shutter does to FPV footage

While the sensor is still reading the lower rows, the camera has already moved on from where it
was when it read the top rows. In a fast yaw, vertical lines lean; in a fast roll, the whole frame
bends; with vibration, the picture wobbles like jelly. This is rolling shutter, and FPV flying,
with its quick rolls and motor vibration, triggers all of it.

Stabilization alone does not remove it. Rotating the whole frame by one angle cannot straighten
an image whose top and bottom were captured at different angles.

## How FPVFlow corrects it

Instead of one rotation per frame, FPVFlow computes a separate rotation for every row of the
frame. Each row is corrected with the camera orientation at the moment that row was read, taken
from the motion data. The frame's own timestamp is treated as the middle of the readout, so the
centre row gets the same correction as without rolling shutter correction, and the rows above and
below are corrected relative to it.

For this to work, FPVFlow needs two values:

- **Frame readout time**: how long the sensor takes to read one frame, from the first row to the
  last, in milliseconds. It cannot be longer than one frame, so the slider stops at the frame
  duration (16.7 ms at 60 fps, 8.3 ms at 120 fps).
- **Frame readout direction**: which way the rows are read relative to the image.

Both need the video and the motion data to be well synchronized. With a poor sync, the per-row
correction adds wobble instead of removing it.

## The settings

The controls are in the **Stabilization** panel, under **Advanced**:

- **Rolling shutter correction** turns the correction on or off.
- **Frame readout time** sets the readout time in milliseconds.
- The arrow next to the slider is the readout direction. Click it to cycle through **Top to
  bottom**, **Bottom to top**, **Left to right** and **Right to left**; hover over it to see the
  current one.

{: .tip }
**No smoothing** with **Rolling shutter correction** on removes only the jello and keeps the
original camera motion. See [Smoothing algorithms]({{ '/technical/smoothing.html' | relative_url }}).

### Readout direction

Most cameras read top to bottom. The direction is relative to the image as it is stored in the
file, so it only changes when the camera itself turns the image around:

- Camera mounted upside down with image flip turned on in the camera: **Bottom to top**.
- Camera that stores portrait frames from a sensor turned sideways: **Left to right** or **Right
  to left**, depending on which way it was turned.

If the jello gets worse when you turn the correction on, the direction or the readout time is
wrong.

## Where the readout time comes from

FPVFlow fills in the readout time by itself when it can, in this order:

1. **From the clip.** Some cameras write their readout time into the file next to the motion
   data. Several DJI, Insta360, Sony and Blackmagic models do. When FPVFlow finds it, the
   correction is turned on with that value when the clip loads.
2. **From the lens profile.** A lens profile can carry a readout time. Loading such a profile
   sets the value and turns the correction on.
3. **Manually**, or with an estimate (see below).

GoPro is the exception: when a GoPro records its readout time, the camera has already corrected
the rolling shutter in the video, so FPVFlow does not use that value.

A Betaflight blackbox log does not know anything about the camera, so when you stabilize a
separate camera (a naked GoPro, a RunCam or similar) with blackbox data, you need to set the
readout time yourself or use a lens profile that includes it.

{: .note }
The default synchronization method, rs-sync, also relies on the readout time. Set it before you
run the automatic sync, not after.

## Estimating the readout time

FPVFlow can estimate the readout time from the footage. In the timeline, move to a part of the
clip with strong camera motion, ideally a fast horizontal pan or yaw, right-click and choose
**Estimate rolling shutter here**. The clip must already be synced.

The estimate is experimental and the app warns that it may be wrong. Check the result on a fast
section of the clip: straight lines should stay straight while the camera turns.

## Measuring it yourself

If you want an exact value for a camera you fly often, you can measure it once and save it in a
lens profile:

1. Point the camera at a light that flickers at a known rate. Many mains-powered LED and
   fluorescent lights flicker at twice the mains frequency (100 Hz on 50 Hz mains, 120 Hz on
   60 Hz). An LED driven by a microcontroller or a signal generator lets you pick the rate.
2. Record a few seconds with a short shutter speed. The flicker shows up as horizontal bright and
   dark bands.
3. Count how many full bright-plus-dark cycles fit from the top to the bottom of the frame.
4. The readout time is the number of cycles divided by the flicker rate. For example, 1.5
   cycles at 100 Hz is 1.5 / 100 s = 15 ms.

The lens calibrator has its own **Rolling shutter correction** and **Frame readout time**
fields, so the value you measure can be stored in the lens profile you create for that camera
and resolution.

{: .warning }
The readout time depends on the camera mode. The same camera can read out much faster at a lower
resolution or a higher frame rate, so measure each mode you use.
