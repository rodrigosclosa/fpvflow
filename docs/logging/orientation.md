---
layout: default
title: IMU orientation
parent: Motion data logging
nav_order: 3
description: >-
  How the IMU orientation string works in FPVFlow (Gyroflow), how to find the right one for an
  external gyro logger, and when to use Rotation instead.
---

# IMU orientation
{: .no_toc }

A gyro chip measures rotation around its own three axes. How those axes line up with the camera
depends on how the chip, the board or the logger is mounted. The **IMU orientation** field in
**Motion data** tells FPVFlow how to turn the logger's axes into the camera's axes. When it is
wrong, stabilization moves the image the wrong way, often making the shake worse instead of
removing it.

1. TOC
{:toc}

---

## The camera axes FPVFlow uses

FPVFlow (like Gyroflow) describes motion in the camera's own frame:

| Axis | Points | Rotation around it | Chart trace |
|:-----|:-------|:-------------------|:------------|
| X | up | yaw (pan left and right) | Y |
| Y | to the left | pitch (tilt up and down) | P |
| Z | backward, out of the back of the camera | roll | R |

The lens looks along the negative Z axis. You can see this frame in **Motion data** with
**Orientation indicator** on: X is drawn in red, Y in green and Z in blue.

In the timeline, with the chart display mode set to **Gyroscope** (right-click the timeline), the
three traces are labelled **Y**, **P** and **R** for yaw, pitch and roll. They show the data after
the orientation has been applied, so they are the quickest way to check it.

## Reading the string

The string has three letters, one for each camera axis in the order X, Y, Z.

- **The position** is the camera axis: first letter for X, second for Y, third for Z.
- **The letter** is the logger axis that feeds it.
- **The case** is the sign: uppercase keeps the value, lowercase inverts it.

So `ZYx` means: camera X takes the logger's Z, camera Y takes the logger's Y, and camera Z takes
the logger's X with the sign flipped. Each letter should appear once. With six ways to order the
letters and two signs for each, there are 48 possible strings.

The same string is applied to the accelerometer, so both stay consistent.

## Defaults

You only need to touch the field when the default is wrong for your setup.

- **Cameras with built-in gyro** (DJI, GoPro, Sony, Insta360 and so on): the orientation comes with
  the clip. Leave it alone.
- **Betaflight blackbox:** `ZYx`, for a flight controller aligned with the quad and a camera looking
  forward. See [Betaflight blackbox]({{ '/logging/betaflight.html' | relative_url }}).
- **.gcsv logs:** the `orientation` line in the file header, or `xzY` if the header has none. See
  [The .gcsv format]({{ '/logging/gcsv.html' | relative_url }}).
- **RunCam and similar gyro CSV files:** a preset per camera model.

{: .note }
Loading a motion data file resets the field to the default of that format. A value you typed is
saved in the project, but not carried over to the next file you load.

## Find the right orientation

### Let FPVFlow guess it

FPVFlow can try all 48 strings and keep the one that best matches the motion it sees in the video.

1. Load the video and the log, and make sure a lens profile is loaded.
2. Get the sync roughly right first: set **Rough gyro offset** in **Synchronization** if the offset
   is large, and check that **Sync search size** covers the rest.
3. Move the playhead to a section with clear camera motion on all axes, without fast objects
   crossing the frame.
4. Right-click the timeline and choose **Guess IMU orientation here**. FPVFlow fills in the
   **IMU orientation** field with the best match.
5. Run **Auto sync** again and check the stabilized preview.

If the guess looks wrong, try it on another section with more varied motion.

### Work it out from the mounting

If the logger has its axes printed on the board (most IMU modules and flight controllers do), you
can write the string by hand. For each camera axis, ask which logger axis points the same way, or
exactly the opposite way.

Example: a logger mounted with its X arrow pointing where the lens looks, Y to the left and Z up.

- Camera X points up. The logger's Z points up too: first letter `Z`.
- Camera Y points left. The logger's Y points left: second letter `Y`.
- Camera Z points backward. The logger's X points forward, the opposite: third letter `x`.

The result is `ZYx`.

### Check it with a test clip

A short test recording settles any doubt, and also tells you whether the orientation or the sync is
the problem.

1. Record a few seconds still, then pan left and right, pause, tilt up and down, pause, roll left
   and right. Keep each movement on one axis.
2. Load the clip and the log, and set the chart display mode to **Gyroscope**.
3. The pan must show up on the **Y** trace only, the tilt on **P** only and the roll on **R** only.
   If a movement lands on the wrong trace, swap those letters in the string.
4. When every movement is on the right trace, sync and play the stabilized preview. If one axis gets
   more shaky instead of calmer, its sign is wrong: change the case of that letter.

## Orientation or Rotation?

**IMU orientation** only swaps and flips axes, in steps of 90 degrees. Any other angle goes into
**Rotation** in the same panel:

- a camera tilted up on the quad: **Pitch**,
- a logger mounted at an angle on the camera: **Pitch**, **Roll** or **Yaw** as needed.

Set the orientation first so the axes are right, then add the rotation for the remaining angle.
Right-click the **Rotation** checkbox and choose **Separate accelerometer rotation** if the
accelerometer needs a different rotation from the gyro.
