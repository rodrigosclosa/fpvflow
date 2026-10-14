---
layout: default
title: Guide
has_children: true
description: "The FPVFlow user guide: quick start, workflows and project files, synchronization, lens calibration, a reference of every panel and the keyboard shortcuts."
nav_order: 4
---

# Guide

The pages in this section explain how the stabilization side of FPVFlow works. FPVFlow keeps the
Gyroflow 1.6.3 engine and interface, so most of what is described here is shared with Gyroflow.
Where FPVFlow adds something, the guide points to the page about it.

| Page | What it covers |
|---|---|
| [Quick start]({{ '/guide/quickstart.html' | relative_url }}) | The shortest path from a clip to a stabilized export. |
| [Workflows and project files]({{ '/guide/workflow.html' | relative_url }}) | Where the motion data comes from, `.gyroflow` project files, presets and batch processing. |
| [Panel reference]({{ '/guide/settings.html' | relative_url }}) | What every control in every panel does. |
| [Synchronization]({{ '/guide/synchronization.html' | relative_url }}) | Auto sync, sync points, offsets and the sync settings. |
| [Lens profiles and calibration]({{ '/guide/calibration.html' | relative_url }}) | The lens profile database and how to calibrate your own lens. |
| [Keyboard shortcuts]({{ '/guide/shortcuts.html' | relative_url }}) | Every shortcut, plus the mouse controls of the timeline. |

The features FPVFlow adds on top of Gyroflow have their own pages:

- [Colour grading]({{ '/colour-grading.html' | relative_url }}) with `.cube` LUTs, applied during the export.
- [External audio]({{ '/external-audio.html' | relative_url }}), a separate audio track that syncs itself to the
  video.
- [DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}), which removes the false rotations some O4 air
  units record.
- [Exporting and the render queue]({{ '/export.html' | relative_url }}).
