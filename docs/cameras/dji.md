---
layout: default
title: DJI
parent: Cameras
nav_order: 1
description: >-
  Stabilize DJI O3 and O4 air unit, Avata, Action and Neo footage in FPVFlow: recording settings
  (RockSteady and HorizonSteady off), automatic lens profile, no sync needed, and the O4 Gyrogate
  repair.
---

# DJI
{: .no_toc }

Recent DJI cameras write their motion data straight into the video file, together with the lens
calibration and the sensor readout time. For FPV pilots this is the simplest workflow there is:
open the clip, check the preview, export.

1. TOC
{:toc}

---

## Supported models

Following the Gyroflow 1.6.3 list:

| Group | Models |
|:------|:-------|
| FPV air units | O3 Air Unit, O4 Air Unit (O4 Lite), O4 Air Unit Pro |
| FPV drones | Avata, Avata 2 |
| Action cameras | Action 2, 4, 5 and 6, Nano |
| Drones | Neo, Neo 2 |

Other DJI products that store the same kind of metadata may load as well. Open a test clip and
check **Contains gyro** in **Video information**.

## What the file gives you

- **Camera orientation, aligned with the frames.** DJI records the orientation the camera
  computed (as quaternions), not raw gyro samples, and it is already in sync with the video. FPVFlow
  collapses the **Synchronization** panel for these clips. If you add sync points anyway, it
  warns that they are not needed and can make the output look worse.
- **The lens profile.** It is built from the calibration stored in the clip, so there is nothing
  to pick in **Lens profile**.
- **The frame readout time**, used for rolling shutter correction.

In **Motion data**, **Integration method** is set to **None**, which means "use the built-in
quaternions". Keep it that way for DJI clips.

## Recording settings

- **RockSteady off and HorizonSteady off.** With either one on, the camera stabilizes and crops
  the image itself, and the clip is not suitable for gyro stabilization.
- **Widest field of view.** Use the wide FOV setting, and 4:3 where your unit offers it, to keep
  the most margin for the crop.
- **Colour.** D-Log M keeps more highlight and shadow detail. Grade it in FPVFlow with a `.cube`
  LUT, see [Colour grading]({{ '/colour-grading.html' | relative_url }}).

### O4 Air Unit and O4 Air Unit Pro

{: .important }
Some O4 units record false rotations in their motion data under vibration (the "Gyrogate"
problem): the stabilized video jerks and then slowly sways, although the raw footage looks fine.
FPVFlow detects this when you open the clip and repairs the motion data. See
[DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}).

The repair needs the 1 kHz attitude track that the O4 records only with **RockSteady and
HorizonSteady both off**. One more reason to fly with them off.

### Vibration

The motion data comes from a gyro inside the camera module. On a drone, motor and prop vibration
reaches it through the frame. If the stabilized video shows fine jitter that is not in the raw
footage, isolate the camera from the frame (a TPU mount instead of a hard plate) and check the
flight controller's filtering. Smooth motors and clean props help the footage as much as the
flight.

## Long recordings

DJI Action cameras split long recordings into several files. When you open the first part of a
recording with a name of the form `DJI_xxxx_yyy.MP4`, FPVFlow looks for the following parts and
offers to join them ("Split recording has been detected"). Files named in other patterns are not
detected; open them one at a time.

Answering **Yes** writes a new, joined file next to the parts, with `_joined` added to the name,
and opens it. Make sure the drive has room for a second copy of the recording.

## Using DJI motion data for another camera

Because a DJI clip carries the camera orientation, a DJI action camera or air unit can serve as
the motion logger for a second camera mounted rigidly next to it:

1. Open the video of the other camera.
2. In **Motion data**, click **Open file** and choose the DJI clip.
3. Keep **Integration method** on **None**.
4. Load a lens profile for the other camera.
5. Run **Auto sync**. This time sync is needed, because the two cameras do not share a clock.
   If it fails, increase **Sync search size** so the search covers the real start offset.

To see the orientation data in the timeline, right-click the chart and choose **Quaternions** in
**Chart display mode**.
