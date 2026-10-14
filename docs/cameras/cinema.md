---
layout: default
title: Cinema cameras
parent: Cameras
nav_order: 7
description: >-
  Stabilize Blackmagic RAW (.braw) and RED (.R3D) footage with FPVFlow: the external SDK
  download, preview, REDline conversion to ProRes, and the project-file workflow with video editor
  plugins.
---

# Cinema cameras
{: .no_toc }

Blackmagic and RED cameras that record motion data store it inside their RAW files. FPVFlow reads
it and can play the RAW footage directly, but it needs the camera maker's SDK to decode the
image, and it does not render RAW files the way it renders MP4 or MOV.

1. TOC
{:toc}

---

## The external SDK

The Blackmagic RAW SDK and the RED SDK are not bundled with FPVFlow. The first time you open a
`.braw` or `.R3D` file, FPVFlow asks:

> This format requires an external SDK. Do you want to download it now?

Answer **Yes**. The download happens once; after that, RAW files open like any other clip. The
SDKs are fetched from Gyroflow's servers, so the first open needs an internet connection.

To keep the preview responsive, FPVFlow lowers the preview resolution to 1080p when you open a
RAW file, unless you have chosen a preview resolution yourself.

## Blackmagic RAW

Blackmagic cameras with gyro logging write the motion data into the `.braw` file. For most models
it is aligned with the frames, so no synchronization is needed and FPVFlow collapses the
**Synchronization** panel. Load or check the lens profile as usual.

`.braw` cannot be rendered from FPVFlow. If you click **Export**, FPVFlow tells you the format is
not available for rendering. Use the project workflow below instead.

## RED

RED cameras that log motion data store it in the `.R3D` file. The Gyroflow 1.6.3 list names the
RED V-RAPTOR and KOMODO. The motion data is aligned with the frames, so no synchronization is
needed when you open the `.R3D` itself.

### Opening R3D clips

- Keep all the `.R3D` parts of a clip together in their folder and open any one of them. FPVFlow
  finds the other parts by their shared name and reads the motion data of the whole clip. You can
  also drag the `.RDC` folder onto the video area.
- If you transcode the clip to ProRes yourself and keep the transcoded file next to the `.R3D`
  with the same name, FPVFlow finds the `.R3D` and reads the motion data from it when you open the
  transcode.

### Rendering R3D

FPVFlow can render R3D clips only through REDline, RED's command-line tool that comes with
REDCINE-X PRO. When REDline is installed with REDCINE-X PRO in its default location:

1. FPVFlow converts the R3D to ProRes and then stabilizes the ProRes.
2. Choose the ProRes flavour in **Advanced** > **Format for R3D conversion**. The colour
   options next to it set how REDline develops the image.

Without REDline, R3D cannot be rendered from FPVFlow; use the project workflow below.

### Settings to check

- **Gyro bias.** A slow, steady drift in the stabilized result usually means the gyro reads a
  small rotation even at rest. The **Gyro bias** fields in **Motion data** correct it. To have
  FPVFlow measure it, use **Estimate gyro bias here** from the timeline's right-click menu at a
  point where the camera sat still.
- **Rolling shutter.** The KOMODO has a global shutter, so **Rolling shutter correction** is not
  needed for it.

## The project workflow for RAW

The way to keep the full quality of RAW footage is to stabilize it inside your editor:

1. Set up the clip in FPVFlow: lens profile, motion data, stabilization.
2. Save the project with <kbd>Ctrl</kbd>+<kbd>S</kbd>. This writes a `.gyroflow` file next to
   the clip.
3. In your editor, apply the Gyroflow plugin to the original RAW clip and load that project file.

On Windows and macOS, when DaVinci Resolve or an Adobe editor is installed, a
**Video editor plugins** section at the bottom of the right panel installs or updates the
Gyroflow plugins.

{: .note }
The plugins are Gyroflow's. They apply the stabilization from the project file, but not
FPVFlow's additions such as the [LUT]({{ '/colour-grading.html' | relative_url }}) or the
[external audio]({{ '/external-audio.html' | relative_url }}). Grade and mix in the editor.
