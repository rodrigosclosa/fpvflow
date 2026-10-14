---
layout: default
title: Betaflight blackbox
parent: Motion data logging
nav_order: 1
description: >-
  Use a Betaflight flight controller blackbox log as the gyro source in FPVFlow: what to log, how
  to load .bbl, .bfl and .csv files, the camera angle, IMU orientation and synchronization.
---

# Betaflight blackbox
{: .no_toc }

A flight controller measures rotation with its gyro hundreds or thousands of times per second to
keep the quad in the air. With blackbox logging on, it writes those readings to flash or to an SD
card, and FPVFlow can use them to stabilize footage from a camera that records no motion data of
its own.

1. TOC
{:toc}

---

## Which firmware and files

FPVFlow reads the blackbox format used by Betaflight and the firmwares that share it, in two forms:

| File | Where it comes from |
|:-----|:--------------------|
| `.bbl`, `.bfl` | The raw log, downloaded from the flight controller's flash or copied from its SD card. A `.bbl` can hold several flights. |
| `.csv` | A raw log converted to text by Blackbox Explorer or `blackbox_decode`. |

From the log, FPVFlow only uses the gyro (`gyroADC`) and accelerometer (`accSmooth`) fields.
Everything else (PIDs, motors, RC, GPS and so on) is skipped when loading.

## Two ways to set it up

**The flight controller of the quad.** No extra hardware: the FC already flies with the camera.
This works best when the camera is mounted hard on the frame. A camera on soft mounts or in a TPU
cradle that wobbles moves differently from the FC, and those differences end up in the video.

**A second flight controller on the camera.** Some pilots and camera operators fix a small FC (with
onboard flash or an SD slot) directly to the camera, powered from a battery or the camera, and use
it only as a logger. This also works on gimbals, cars and handheld rigs. Since that FC is never
armed, its blackbox has to be told to log all the time (see below).

## Configure the blackbox

Do this in Betaflight Configurator, in the Blackbox tab or in the CLI.

1. **Logging device:** onboard flash or SD card.
2. **Logging rate:** high enough to follow fast motion. A few hundred hertz up to about 1 kHz is a
   common range for stabilization; higher rates only fill the flash faster.
3. **Debug mode:** set it to none, unless you need it for tuning. Debug values take space and
   FPVFlow does not use them.
4. **Only log what you need** (Betaflight 4.3 and later). Each group of fields can be switched off
   with a `blackbox_disable_...` setting. Type `get blackbox_disable` in the CLI to see them all.
   Keep `blackbox_disable_gyro` and `blackbox_disable_acc` set to `OFF`, and turn on the ones you
   do not need for tuning to make the flash last much longer.
5. **For a logger that is never armed:** set the blackbox mode to log always
   (`set blackbox_mode = ALWAYS`), so it records as soon as it has power.

```
# Example for a dedicated logger FC
set debug_mode = NONE
set blackbox_mode = ALWAYS
set blackbox_disable_motors = ON
set blackbox_disable_rc = ON
save
```

{: .note }
Setting names and accepted values change between Betaflight versions. If a command is rejected,
check `get blackbox` in the CLI of your version.

{: .tip }
The accelerometer is optional for plain stabilization, but keep it in the log. It lets FPVFlow
correct gyro drift and level the horizon.

## Record

- Start the camera, then arm (or power the logger). Stop the camera first, then disarm. The log has
  to cover the whole clip.
- If you can, make a few sharp movements right after starting the recording. They give the
  synchronization an easy pattern to lock on to.
- Download the log after the session: from the Blackbox tab (saving the flash to a file), by
  putting the FC in mass storage mode, or by taking out the SD card.

## Load the log in FPVFlow

1. Open the video.
2. In **Motion data**, click **Open file** and choose the `.bbl`, `.bfl` or `.csv` file. If you give
   the log the same base name as the video and keep both in one folder, FPVFlow picks it up when you
   open the video (for MP4, MOV and MKV videos that carry no motion data of their own).
3. The **Detected format** shows `BlackBox` followed by the firmware revision found in the log.
4. The **Select log** list appears, with **All logs combined** plus one entry per flight in the
   file. Each flight entry shows the log number, its start and end time and its duration. Pick the
   flight that matches the clip, or keep **All logs combined**.

For logs, FPVFlow sets the **Integration method** to VQF by default, which combines the gyro and the
accelerometer.

## Camera angle and orientation

The flight controller measures the rotation of the quad. The camera usually points up at an angle,
so the two are not aligned.

- **Rotation:** check it and set **Pitch** to the camera uptilt. The tooltip in the app describes
  **Pitch** as the camera angle up or down when using FPV blackbox data. If the stabilized image
  looks worse after you enter the angle, try the same value with the opposite sign. Use **Roll** and
  **Yaw** only if the camera is also mounted rolled or turned.
- **IMU orientation:** for blackbox logs FPVFlow fills in `ZYx`. That matches a flight controller
  whose data is aligned with the quad (the board alignment set correctly in Betaflight) and a camera
  that looks forward. A logger FC mounted differently on a camera needs a different string; the
  [IMU orientation]({{ '/logging/orientation.html' | relative_url }}) page explains how to find it.

{: .note }
Loading another log resets **IMU orientation** to the default of that format. Check it again after
switching files.

## Synchronize

The blackbox starts when the FC arms (or when it powers up, for an always-on logger), never at the
exact moment the camera starts. FPVFlow has to find that offset.

1. Make sure a lens profile is loaded in **Lens profile**. Without it, sync results are unreliable.
2. If you know roughly how much earlier or later the log started, enter it in **Rough gyro offset**
   (in seconds). The small checkbox next to the field searches both directions, which doubles the
   time it takes; use it for the first sync point and turn it off when the offset is known.
3. Raise **Sync search size** if the offset can be large, for example when the camera ran for a
   while before you armed.
4. Click **Auto sync**. FPVFlow analyses several points along the clip and places sync points on
   the timeline.
5. Play the stabilized clip from start to end. Where a section looks off, right-click the timeline
   there and choose **Auto sync here**, or **Add manual sync point here** to set it by hand.

## Cleaning up the data

Blackbox data is usually clean, but a few options in **Motion data** help with noisy or damaged
logs:

- **Low pass filter** smooths high-frequency noise from props and motors. Lower cutoff values mean
  more filtering.
- **Glitch filtering** detects and repairs short bursts of corrupt gyro samples.
- **Gyro bias** removes a constant drift. Right-click the timeline on a section where the camera is
  still and choose **Estimate gyro bias here** to fill it in. It averages the gyro over about
  0.4 seconds around that point, so do it after syncing and on a moment with no motion at all.

## Related pages

- [IMU orientation]({{ '/logging/orientation.html' | relative_url }})
- [Other loggers]({{ '/logging/other-loggers.html' | relative_url }})
- [Getting started]({{ '/getting-started.html' | relative_url }})
