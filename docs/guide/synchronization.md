---
layout: default
title: Synchronization
parent: Guide
nav_order: 4
description: "Synchronize motion data with the video in FPVFlow: when syncing is needed, how Auto sync works, how to add, check and fix sync points, and what each setting in the Synchronization panel does."
---

# Synchronization
{: .no_toc }

Stabilization only works if FPVFlow knows which gyro sample belongs to which video frame. This page
explains how that link is made and how to repair it when it is wrong.

1. TOC
{:toc}

---

## Do you need to sync at all?

| Motion data | Sync needed? |
|---|---|
| Built into the video and already timed to the frames (DJI air units and action cameras, recent GoPro models and others) | No. FPVFlow folds the **Synchronization** panel away. |
| Built into the video but not frame-accurate | Yes, usually a quick **Auto sync**. |
| A separate log (Betaflight blackbox, ArduPilot, a phone app, a second camera) | Yes, always. |

When a clip already carries synced motion data and you add sync points anyway, the panel warns:
**This file uses synced motion data, additional sync points are not needed and can make the output
look worse.** Delete the points if you see it.

FPVFlow also checks before an export: if the clip needs sync points and has none, it asks whether
you really want to render.

{: .note }
The [external audio track]({{ '/external-audio.html' | relative_url }}) has its own, separate synchronization
(**Auto-sync audio**). Everything on this page is about the motion data.

## What a sync point is

A sync point is a moment in the clip together with the measured time difference (the offset, in
milliseconds) between the motion data and the video at that moment. Sync points appear as small
markers on the timeline, each labelled with its offset.

- With one sync point, that offset is used for the whole clip.
- With two or more, the offset is interpolated in a straight line between neighbouring points, and
  held at the first and last value before the first point and after the last one.

Several points are useful because the clocks of two devices drift apart slowly, so the offset at the
end of a long flight is not the same as at the start.

The marker colour shows how well a point agrees with the others. FPVFlow fits a straight line
through all the offsets. A point close to that line is green, and it turns towards red as it moves
away from it (fully red at 30 ms, or 300 ms for RunCam footage). A single red point among green ones
is usually a bad measurement.

## Auto sync

1. Make sure the lens profile is loaded. Without it the analysis is likely to be wrong, and FPVFlow
   asks before it continues.
2. If you only want part of the clip, set the trim range first (<kbd>I</kbd> and <kbd>O</kbd>).
   The sync points are spread over the trimmed part.
3. Click **Auto sync** in **Synchronization**.

FPVFlow places **Max sync points** evenly over the clip (or over each trim range), analyzes a short
piece of video around each one, compares the camera motion it sees in the image with the motion
data, and stores the best match as a sync point.

### Sync a single moment

Move the playhead to a part of the flight with clear camera motion, then right-click the timeline
and choose **Auto sync here**, or press <kbd>A</kbd>. Good moments have quick but not blurry
rotation and a detailed, well-lit scene. Bad moments are hovering on the ground, flying into the
sun, propellers filling the frame, or a fast flip that smears the image.

### Experimental point selection

The small checkbox right next to the **Auto sync** button enables an experimental mode in which
FPVFlow chooses the sync moments itself instead of spacing them evenly.

## Check the result

1. Play the stabilized clip and watch for wobble or a horizon that lags behind the motion.
2. Look at the colours of the sync point markers.
3. Turn on the **S** axis button next to the chart (tooltip **Sync points debug layer**). The chart
   then also draws the rotation FPVFlow measured from the video around each sync point, so you can
   compare it with the motion data curves.

To look closer at one point, double-click its marker, or right-click it and choose **Zoom in**.
**Zoom in and loop** also sets a trim range around it and plays that piece in a loop.

## Fix a bad sync point

- **Delete it.** Right-click the marker and choose **Delete sync point**. If the other points are
  good, this is often all that is needed.
- **Measure it again** somewhere else with **Auto sync here**.
- **Adjust it by hand.** Click the marker (or right-click it and choose **Edit offset**). A slider
  and a number field appear under the timeline. Change the offset while you watch the preview, then
  click **Save**.
- **Add a manual point.** Right-click the timeline and choose **Add manual sync point here**, or
  press <kbd>P</kbd>. The new point starts at the offset already in use at that moment, and the
  editor opens so you can fine-tune it.
- **Start over.** Right-click the timeline and choose **Delete all sync points**.

{: .tip }
For manual tuning, zoom into a short piece with a sharp motion and play it in a loop. When the
offset is right, the stabilized image stays still through the motion. When it is wrong, the image
jerks at the start or at the end of the motion.

## When auto sync keeps failing

### The log starts much earlier or later than the video

Auto sync only searches a window of a few seconds around the expected offset (**Sync search size**,
5 s by default). If the flight controller started logging at arm and you started recording much
later, the real offset may be outside that window.

- Enter the approximate difference in **Rough gyro offset**, in seconds.
- If you do not know in which direction the difference goes, tick the small checkbox next to the
  field. FPVFlow then tries both positive and negative offsets, which takes twice as long. Untick it
  once you know the offset.
- Or increase **Sync search size**.

### The motion looks mirrored or on the wrong axis

The axes of the logging device do not match the camera. Right-click the timeline at a moment with
clear motion and choose **Guess IMU orientation here**. FPVFlow fills in **IMU orientation** in the
**Motion data** panel. See [Motion data]({{ '/guide/settings.html' | relative_url }}#motion-data).

### The log is noisy or drifts

- Try the **Low pass filter** in **Motion data**.
- A gyro that reads a small rotation while the camera is still has a bias. Right-click the timeline
  at a moment where the camera sits still and choose **Estimate gyro bias here**. FPVFlow fills in
  **Gyro bias** in **Motion data**.

### Slow motion or high frame rates

Editing **Frame rate** in **Video information** also sets **Analyze every n-th frame**: FPVFlow
divides the new frame rate by the original one and uses the whole part of the result (never less
than 1). You can still change it by hand afterwards.

## The Synchronization panel

| Control | What it does |
|---|---|
| **Auto sync** | Adds sync points automatically, as described above. The small checkbox next to it enables the experimental point selection. |
| **Rough gyro offset** | Starting guess for the offset, in seconds. The checkbox next to it searches both directions. |
| **Sync search size** | How far around the rough offset to search, in seconds (default 5 s). When **Offset method** is not **Essential matrix**, a checkbox next to it makes FPVFlow find the offset quickly with the essential matrix first and then refine it. It is ticked automatically for search sizes above 10 s. |
| **Max sync points** | How many points **Auto sync** adds (default 3). |

Under **Advanced** in the same panel:

| Control | What it does |
|---|---|
| **Analyze every n-th frame** | Skip frames during the analysis. 1 analyzes every frame. |
| **Time to analyze per sync point** | Length of video analyzed around each point (default 1.5 s). Longer can be more reliable but is slower. |
| **Processing resolution** | Resolution the video is scaled to for the analysis (default 720p). |
| **Optical flow method** | How motion is tracked in the image: **OpenCV (DIS)** (default), **OpenCV (PyrLK)** or **AKAZE**. AKAZE can be more accurate but is much slower. Use it only if OpenCV fails. |
| **Pose method** | How the camera rotation is estimated from the tracked points (default `findEssentialMat`). FPVFlow switches to `Almeida` by itself for lenses with a very long focal length. |
| **Offset method** | How the offset is computed. **Essential matrix** matches the rotation seen in the video to the gyro. **Visual features** picks the offset at which the stabilized image moves the least. **rs-sync** (default) also models rolling shutter, so set the rolling shutter correctly first. |
| **Low pass filter** | When ticked, removes motion above the cutoff you set (in Hz) from the rotation measured in the video, before it is compared with the gyro. Off (0 Hz) by default. |
| **Show detected features** / **Show optical flow** | Draw the tracked points and their motion on the preview during the analysis. |

The sync settings are saved in the project file and can be part of a [preset]({{ '/guide/workflow.html' | relative_url }}#presets).
A lens profile can also carry sync settings, which are applied when the profile loads.
