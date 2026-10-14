---
layout: default
title: Video editor plugins
description: >-
  Use FPVFlow with the Gyroflow plugins for DaVinci Resolve, Adobe Premiere Pro, After Effects and
  Final Cut Pro: what carries over from FPVFlow, what does not, and how to use the Gyrogate
  corrected copy in your editor.
nav_order: 10
---

# Video editor plugins
{: .no_toc }

The Gyroflow project maintains plugins that stabilize a clip directly inside your video editor,
without rendering a new file first. They work with FPVFlow clips and projects, with some limits.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## The Gyroflow plugins

The Gyroflow team builds and releases two plugins in the
[gyroflow-plugins](https://github.com/gyroflow/gyroflow-plugins) repository, where you also find
downloads and instructions:

- **OpenFX**, for DaVinci Resolve and other OpenFX hosts.
- **Adobe**, for Premiere Pro and After Effects.

For **Final Cut Pro** there is Gyroflow Toolbox
([gyroflowtoolbox.io](https://gyroflowtoolbox.io)), a separate third-party plugin by LateNite
Films. FPVFlow cannot install or update it; get it from its own site.

None of these plugins is made by FPVFlow. They run Gyroflow's own stabilization engine inside
the editor.

Working inside the editor is useful when you want to keep the original file in your timeline,
grade from the camera's log footage in the editor, or avoid an extra render. For a quick
stabilized and graded clip, exporting from FPVFlow is simpler; see
[Exporting and the render queue]({{ '/export.html' | relative_url }}).

## Installing and updating from FPVFlow

On Windows and macOS, when FPVFlow detects DaVinci Resolve (or another OpenFX host) or Adobe
Premiere Pro or After Effects on your computer, it shows a **Video editor plugins** section in
its side panel. For the Adobe and OpenFX plugins it shows the installed version, or `---` if the
plugin is not installed, and an **Install** or **Update** button that downloads the latest
release from the gyroflow-plugins repository. **More information** opens the repository page.

If FPVFlow does not detect your editor, the section does not appear: download the plugin from
the repository and follow its instructions.

## Sending a project to the editor

The plugins read the same project files that FPVFlow saves (`.gyroflow`). Set up the clip in
FPVFlow as usual (sync, lens profile, smoothing), then save the project:

- **Ctrl+S**, or **Export project file (including gyro data)** in the menu next to the
  **Export** button, saves the settings together with the motion data. Use it when the motion
  data comes from a separate file, such as a Betaflight blackbox log, or when the project will be
  opened on another computer.
- **Export project file** in the same menu saves only the settings.

In the editor, add the Gyroflow effect to the clip and load that project file in the plugin.

## What carries over and what does not

The plugins use the Gyroflow engine, so only Gyroflow's settings reach the editor: sync, lens
profile, smoothing, horizon lock, zooming and rolling shutter correction. The features FPVFlow
adds are not part of the plugins:

| FPVFlow feature | In the plugins |
|:----------------|:---------------|
| [Colour grading]({{ '/colour-grading.html' | relative_url }}) (LUT and colour adjustments) | No. Grade in the editor instead. |
| [External audio]({{ '/external-audio.html' | relative_url }}) | No. Put the audio track in the editor's timeline. |
| [Gyrogate repair]({{ '/gyrogate.html' | relative_url }}) setting in the project | No. Use the corrected copy below. |

{: .note }
FPVFlow's engine also includes a few Gyroflow features that are newer than Gyroflow 1.6.3:
**Glitch filtering**, and **Automatic lock** and **Lock pitch angle** for horizon lock. Whether
the plugin applies them depends on the plugin version; an older plugin may not. See
[Technical details]({{ '/technical/' | relative_url }}).

### DJI O4 clips with Gyrogate

The **Remove DJI Gyrogate false rotations** setting only exists in FPVFlow, so a plugin
stabilizing the original O4 clip still sees the false rotations.

To get the repaired motion into your editor, click **Save corrected copy** in FPVFlow's
**Motion data** panel. FPVFlow writes `<name>_gyrofix.MP4` next to the original, with the
orientation track repaired and the video and audio unchanged. Use that copy in the editor and
stabilize it with the plugin like any other DJI clip.

{: .tip }
Record your O4 clips with RockSteady and HorizonSteady off. The repair needs the full-rate motion
data that the O4 only records with in-camera stabilization off, so a clip recorded with them on
cannot get a corrected copy.

## Camera motion for VFX

If you need the camera motion itself, for example to match a 3D scene or add tracked graphics,
the **Motion data** panel has **Export camera data (CSV/JSON/USD/AE)**, which writes the camera
orientation in formats that compositing and 3D tools can import.
