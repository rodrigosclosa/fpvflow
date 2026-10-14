---
layout: default
title: GoPro
parent: Cameras
nav_order: 2
description: >-
  Stabilize GoPro HERO5 and later footage in FPVFlow: GPMF motion data, HyperSmooth off, lens
  modes and profiles, sync on older models, and joining split recordings.
---

# GoPro
{: .no_toc }

Every GoPro from the HERO5 on writes its gyro and accelerometer readings into the video file, in
GoPro's GPMF metadata track. FPVFlow reads it directly, so there is no separate log to manage.

1. TOC
{:toc}

---

## Newer and older models

GoPro models differ in what they record:

- **Models that record per-frame camera orientation** (the newer HERO generations) give FPVFlow
  orientation that is already aligned with the frames. These clips need no synchronization, and
  FPVFlow collapses the **Synchronization** panel for them.
- **Models that only record raw gyro and accelerometer samples** need **Auto sync** after the
  lens profile is loaded, like any other raw gyro source.

You do not have to know which kind you have: if the **Synchronization** panel opens collapsed,
the clip needs no sync points.

For vibration on a drone, see the general advice in [Cameras]({{ '/cameras/' | relative_url }}).

## Lens profile

GoPro records which lens mode was used (Wide, Linear, SuperView, HyperView, Max and others) and
whether HyperSmooth was on. FPVFlow uses that to pick the matching profile from the built-in
database. Newer cameras also store their own lens calibration in the file, and FPVFlow builds the
profile from it, including the horizontal stretch of SuperView and HyperView.

Check **Detected lens** in **Video information** and the profile name in **Lens profile**. If
the profile does not match your resolution or aspect ratio, a warning appears under it.

## Recording settings

- **HyperSmooth off.** It crops and stabilizes the image in camera. Some models also record the
  correction they applied, so FPVFlow can still process such clips, but you get the most margin
  and the cleanest result with HyperSmooth off. The same goes for in-camera horizon levelling.
- **Wide lens, 4:3 or 8:7.** The full-sensor modes keep the most image for the stabilization
  crop. Crop to 16:9 at export.
- **Shutter.** Auto shutter on a bright day gives very short exposures and choppy motion after
  stabilization. Use an ND filter, or set the shutter in Protune, if you want more motion blur.
- **Colour.** GoPro Flat (log) footage can be graded with a LUT in FPVFlow, see
  [Colour grading]({{ '/colour-grading.html' | relative_url }}).

## Long recordings

GoPro splits long recordings into chapter files. When you open the first chapter, FPVFlow
detects the rest ("Split recording has been detected") and offers to join them. Answering
**Yes** writes a new, joined file next to the chapters, with `_joined` added to the name, and
opens it. Make sure the drive has room for a second copy of the recording.

## GoPro as a motion logger

A GoPro mounted rigidly on another camera can provide motion data for it. Open the other
camera's video, then in **Motion data** click **Open file** and choose the GoPro clip. Load a
lens profile for the main camera and run **Auto sync**.
