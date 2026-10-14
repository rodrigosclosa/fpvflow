---
layout: default
title: Lens profiles and calibration
parent: Guide
nav_order: 5
description: "Find the right lens profile in the FPVFlow database, understand when a profile fits your footage, and calibrate your own FPV camera lens with the built-in lens calibrator and its chessboard target."
---

# Lens profiles and calibration
{: .no_toc }

1. TOC
{:toc}

---

## Why a lens profile matters

FPV cameras use wide, strongly distorted lenses. To rotate the image correctly, FPVFlow has to know
how the lens maps the world onto the sensor: its focal length (in pixels), its optical centre and
its distortion. A lens profile stores exactly that. With a wrong or missing profile the
stabilization wobbles, straight lines bend while the camera rotates, and synchronization becomes
unreliable. FPVFlow warns when no profile is loaded, both in the preview and before an export.

A profile belongs to one combination of camera, lens and recording mode: the resolution, the aspect
ratio and the field of view setting (for example "Wide" or "4:3"). When you load a profile, the
**Lens profile** panel compares the size it was calibrated at with the size of your video. If only
the size differs, you get a warning that the result may not look correct; if the aspect ratio
differs too, the message becomes an error that the result will not look correct. A frame-rate
difference is not checked at all.

## Finding a profile

### Automatic selection

For many cameras FPVFlow reads the camera model and recording mode from the file and loads the
matching profile by itself. When the profile comes from the camera maker or from the Gyroflow team,
FPVFlow also collapses the **Lens profile** and **Motion data** panels, since there is nothing to
check.

### Searching the database

FPVFlow ships with the lens profile database of the Gyroflow project, made of profiles contributed
by users and camera makers, and checks online for an updated database when it starts.

1. In **Lens profile**, type in the search box: the camera brand, model, or the mode.
2. Profiles whose aspect ratio does not match your video are dimmed.
3. Some profiles carry an **official** tag. The others show a rating from users when one exists.
4. Click the star in front of a profile to make it a favourite. Favourites (and your presets) are
   listed first, followed by the profiles that match the aspect ratio of your video.

When you load an unofficial profile, FPVFlow says it cannot guarantee its correctness and lets you
rate it with **Good** or **Bad** once you have seen the result.

### Loading a file

**Open file** in **Lens profile** loads any profile `.json` file, whether or not it is in the
database. Profiles you export from the calibrator into the default folder also appear in the search
box the next time FPVFlow starts.

### What else a profile can set

Besides the lens itself, a profile can carry:

- a rolling shutter readout time, applied to **Rolling shutter correction** in **Stabilization**;
- a gyro low pass filter frequency, applied in **Motion data**;
- sync settings, applied in **Synchronization**;
- a default output size, applied in **Export settings**.

### Checking and fine-tuning a profile

Under **Advanced** in **Lens profile** you can see and edit the profile's numbers: **Pixel focal
length**, **Focal center** and **Distortion coefficients**. Changes apply right away to the preview.
The same section has **Lens is under water**, which corrects for the different refraction of water,
and **Export STMap**, which writes the lens distortion as an image map for compositing software.

## Calibrating your own lens

If no profile fits, calibrate the lens yourself. You record a known pattern with the camera, and the
calibrator works out the lens parameters from it.

### What you need

- The camera, set to the exact mode you fly with (resolution, aspect ratio, field of view). Every
  mode you use needs its own profile.
- A calibration pattern. The calibrator shows one full screen: open the calibrator (below) and
  click **Open calibration target**. It is a 14 x 8 chessboard with a few dots in the middle. A flat
  monitor works well; the bigger, the better.
- Good, even light, so you can use a short shutter time and avoid motion blur.

### Record the calibration clip

1. Turn off the camera's own stabilization and any lens correction (for example distortion
   correction or "dewarp" modes).
2. Show the pattern full screen on the monitor.
3. Record for about a minute. The calibrator can only learn the distortion in the parts of the
   frame where it saw the pattern, and a wide FPV lens bends the image most near the borders. So
   sweep the pattern slowly through the whole picture, borders and corners included, while you
   also tilt the camera on both axes and change the distance now and then.
4. Keep the whole pattern inside the frame at all times, and move slowly so the squares stay
   sharp.

{: .tip }
The edges and corners of the frame matter most for a wide FPV lens. A calibration made only with the
pattern in the centre will look fine in the middle and bend at the edges.

### Run the calibrator

1. In **Lens profile**, click **Create new**. The **Lens calibrator** window opens.
2. Drop the calibration clip on it, or open it with **Open file** in its **Video information**
   panel.
3. Click **Auto calibrate**. FPVFlow scans the clip, looks for the pattern, keeps the sharpest
   detections (up to **Max calibration points**, 15 by default), and computes the lens. The progress
   shows how many good frames were found and how sharp the pattern was.
4. Prefer to choose the frames yourself? Move to a frame with a sharp, complete pattern, right-click
   the timeline and choose **Add calibration point**. Calibration points appear as markers on the
   timeline and can be deleted from their right-click menu.

If no pattern is detected, FPVFlow explains why: usually blur or poor focus. The pattern sharpness is
measured in pixels, and frames above **Sharpness limit** (8 px by default, under **Advanced**) are
rejected. Record a sharper clip, or raise the limit carefully.

### Judge the result

- **Reprojection error** in the calibrator is the typical (RMS) mismatch, in pixels, between where the
  pattern corners were detected and where the computed lens puts them. It is shown in green below 1,
  orange below 5 and red above. Aim for less than 1; above 5 the profile is not good enough.
- Check the preview: straight lines of the pattern and of the room should look straight across the
  whole frame, including the corners.
- The **FOV** slider under **Advanced** zooms the preview out so you can inspect the edges.

### Describe and export the profile

1. Fill in the table with the pencil icons: **Camera brand**, **Camera model**, **Lens model**,
   **Camera setting** (the mode, such as "4K 4:3 Wide"), **Additional info**, **Default output
   size** and **Calibrated by**. Some fields are filled in from the file metadata already.
2. If you know the rolling shutter readout time of this mode, enable **Rolling shutter correction**
   and enter the **Frame readout time**. It is stored in the profile. See
   [Rolling shutter](#rolling-shutter) below.
3. Leave **Upload lens profile to the database** ticked to share the profile, or untick it to keep it
   to yourself. Uploading sends it to the Gyroflow project's database, and FPVFlow asks you to agree
   that it is published under the GPLv3 before it is sent.
4. Click **Export lens profile**. The file dialog opens in the folder FPVFlow reads profiles from.
   Saved there, the profile shows up in the search box after the next start of FPVFlow. To use it
   right away, load it with **Open file** in **Lens profile**.

Now load your flight footage, select the new profile in **Lens profile** and check the
stabilization.

### Several cameras at once

Select several calibration clips at once in the calibrator's file dialog and FPVFlow offers to
process them automatically: each clip is calibrated in turn, and a profile is exported next to each
clip whose reprojection error is below 5.

### Calibrator options

Under **Advanced** in the calibrator:

| Control | What it does |
|---|---|
| **FOV** | Zooms the preview in or out. For inspection only. |
| **Analyze every n-th frame** | How many frames **Auto calibrate** skips between checks (default 10). |
| **Sharpness limit** | Frames with a blurrier pattern than this are ignored (default 8 px). |
| **Digital lens** | Use **GoPro Superview** or **GoPro Hyperview** when the camera stretches the image digitally in those modes. FPVFlow selects it from the file metadata when it can. |
| **Input horizontal stretch** / **Input vertical stretch** | For footage that is stored squeezed, such as anamorphic recordings. FPVFlow offers to update the output size to match. |
| **Iterations** | Iterations of the lens solver (default 500). |
| **Low pass filter** | A gyro filter frequency stored in the profile and applied whenever the profile is used. |
| **Focal length** | The lens's native focal length and the sensor **Crop factor**, stored as information in the profile. |
| **Preview resolution** / **Processing resolution** | Resolution used for the preview and for the pattern detection. Detection should run at full resolution or 4K, and FPVFlow warns if you go lower. |
| **Lens is asymmetrical** | For lenses whose optical centre is clearly off the image centre. |
| **Sensor is global shutter** | Marks the profile as having no rolling shutter. |
| **Plain chessboard pattern (previous version without dots in the middle)** | Tick it if you recorded the older pattern without the dots. |

### Footage that is already undistorted

Some cameras correct the lens distortion themselves, and some lenses are rectilinear. For such
footage tick **No distortion, focal length only** in the calibrator and enter the 35 mm equivalent
**Focal length**. The calibrator shows the resulting horizontal and diagonal field of view, and you
can export the profile without recording a pattern. Such profiles cannot be uploaded to the
database.

## Rolling shutter

Most FPV and action cameras read the sensor line by line, so fast rotation skews or wobbles the
image. **Rolling shutter correction** in **Stabilization** (under **Advanced**) undoes this, if it
knows the **Frame readout time** (in milliseconds) and the readout direction. Many official
profiles and some cameras provide the value.

When you make your own profile, store the value in it: the calibrator has its own
**Rolling shutter correction** box with **Frame readout time**, and whatever it holds at export
time goes into the profile, so every clip that loads the profile gets it. If you do not know the
value, FPVFlow has an experimental estimator: with the clip synced, right-click the timeline at a
fast, mostly horizontal motion and choose **Estimate rolling shutter here**. FPVFlow warns that the
result may be wrong, so play that part back and look at vertical edges; if they still lean, adjust
**Frame readout time** by hand, or flip the readout direction when a change makes it worse.

For a precise measurement with dedicated hardware, see the Gyroflow project's
[rollingshutter](https://github.com/gyroflow/rollingshutter) tool.
