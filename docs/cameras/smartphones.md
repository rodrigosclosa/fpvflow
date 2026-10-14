---
layout: default
title: Smartphones
parent: Cameras
nav_order: 6
description: >-
  Use a smartphone with FPVFlow: sensor logging apps for iOS and Android, recording a gyro log
  alongside the phone's video or another camera, lens profiles and synchronization.
---

# Smartphones

Every smartphone has a gyroscope. With a sensor logging app running while you film, FPVFlow can
stabilize the phone's own video, or the video of another camera the phone is strapped to.

## Supported apps

Following the Gyroflow 1.6.3 list:

| Platform | Apps |
|:---------|:-----|
| iOS | [Sensor Logger](https://apps.apple.com/us/app/sensor-logger/id1531582925), [G-Field Recorder](https://apps.apple.com/at/app/g-field-recorder/id1154585693), [Gyro](https://apps.apple.com/us/app/gyro-record-device-motion-data/id1161532981) |
| Android | [Sensor Logger](https://play.google.com/store/apps/details?id=com.kelvin.sensorapp), [Sensor Record](https://play.google.com/store/apps/details?id=de.martingolpashin.sensor_record), [OpenCamera Sensors](https://github.com/MobileRoboticsSkoltech/OpenCamera-Sensors) |

FPVFlow recognizes the log format of each of these apps by itself; you only need to choose the
file.

The Gyroflow list also names [MotionCam Pro](https://play.google.com/store/apps/details?id=com.motioncam.pro)
for Android. Its motion data is not read by a dedicated reader like the apps above, so open its
video in FPVFlow first and check **Contains gyro** in **Video information** before you rely on it.

## Workflow

1. Start the logging app, then start recording video (in the phone's camera app or on the other
   camera). Stop the video first and the log last, so the log covers the whole clip.
2. Copy both files to the computer.
3. Open the video in FPVFlow.
4. In **Motion data**, click **Open file** and choose the log.
5. Load a lens profile: search **Lens profile** for the phone or camera model, or calibrate one
   with **Create new**. Phones have several cameras, so make sure the profile is for the lens you
   filmed with, at the resolution you used.
6. Run **Auto sync**. A phone log never shares a clock with the video, so sync is always needed.

If the horizon or the direction of motion looks wrong after syncing, check **IMU orientation** in
**Motion data**: the phone may have been mounted in a different orientation from the camera.

## Things to watch

- **Optical stabilization.** Many phone cameras have optical image stabilization that cannot be
  switched off. It moves the lens on its own, and that movement is not in the gyro log, so some
  wobble can remain.
- **Video stabilization in the camera app.** Turn it off; it crops and warps the image.
- **Mounting.** When the phone logs for another camera, both must be fixed to each other rigidly.
  Any play between them is motion the log gets wrong.

{: .note }
FPVFlow is a desktop app. To stabilize on the phone itself, use the
[Gyroflow](https://github.com/gyroflow/gyroflow) mobile app.
