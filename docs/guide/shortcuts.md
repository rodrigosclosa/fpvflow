---
layout: default
title: Keyboard shortcuts
parent: Guide
nav_order: 6
description: "All FPVFlow keyboard shortcuts for playback, trimming, sync points, the motion chart, horizon roll, export and the render queue, plus the mouse controls of the timeline."
---

# Keyboard shortcuts
{: .no_toc }

On macOS, use <kbd>Cmd</kbd> where this page says <kbd>Ctrl</kbd>, and <kbd>Option</kbd> for
<kbd>Alt</kbd>. Shortcuts work while the main window has focus.

1. TOC
{:toc}

---

## Playback and navigation

| Keys | Action |
|---|---|
| <kbd>Space</kbd> or <kbd>F3</kbd> | Play / pause (with the external audio track, if one is loaded) |
| <kbd>K</kbd> | Play / pause and reset the playback speed to normal |
| <kbd>L</kbd> | Play forward faster. Each press moves to the next speed in the cycle 1x, 2x, 4x, 8x, 16x. The first press after starting FPVFlow gives 2x; <kbd>K</kbd> resets the cycle, so the next <kbd>L</kbd> starts at 1x |
| <kbd>J</kbd> | Jump back 500 frames and play |
| <kbd>Left</kbd>, <kbd>Page Up</kbd>, <kbd>,</kbd> or <kbd>F2</kbd> | Previous frame |
| <kbd>Right</kbd>, <kbd>Page Down</kbd>, <kbd>.</kbd> or <kbd>F4</kbd> | Next frame |
| <kbd>Ctrl</kbd>+<kbd>Left</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Up</kbd>, <kbd>Ctrl</kbd>+<kbd>,</kbd> or <kbd>F5</kbd> | Back 10 frames |
| <kbd>Ctrl</kbd>+<kbd>Right</kbd>, <kbd>Ctrl</kbd>+<kbd>Page Down</kbd>, <kbd>Ctrl</kbd>+<kbd>.</kbd> or <kbd>F6</kbd> | Forward 10 frames |
| <kbd>Shift</kbd>+<kbd>Left</kbd> or <kbd>Shift</kbd>+<kbd>Page Up</kbd> | Previous keyframe |
| <kbd>Shift</kbd>+<kbd>Right</kbd> or <kbd>Shift</kbd>+<kbd>Page Down</kbd> | Next keyframe |
| <kbd>M</kbd> | Mute / unmute |

## Trimming

| Keys | Action |
|---|---|
| <kbd>I</kbd> or <kbd>[</kbd> | Set the start of the nearest trim range at the playhead |
| <kbd>O</kbd> or <kbd>]</kbd> | Set the end of the nearest trim range at the playhead |
| <kbd>Ctrl</kbd>+<kbd>I</kbd> or <kbd>Ctrl</kbd>+<kbd>[</kbd> | Start a new trim range at the playhead |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> or <kbd>Ctrl</kbd>+<kbd>]</kbd> | End a new trim range at the playhead |
| <kbd>Home</kbd> or <kbd>H</kbd> | Go to the start of the nearest trim range |
| <kbd>End</kbd> or <kbd>;</kbd> | Go to the end of the nearest trim range |
| <kbd>C</kbd> | Clear all trim ranges |

## Synchronization

| Keys | Action |
|---|---|
| <kbd>A</kbd> | **Auto sync here**: measure a sync point at the playhead |
| <kbd>P</kbd> | **Add manual sync point here** |

See [Synchronization]({{ '/guide/synchronization.html' | relative_url }}).

## Preview

| Keys | Action |
|---|---|
| <kbd>S</kbd> | Stabilization on / off, to compare with the original |
| <kbd>D</kbd> | Stabilization overview on / off (zoomed-out view showing the whole stabilized frame) |
| <kbd>V</kbd> | Split view of the stabilization overview |
| <kbd>G</kbd> | Grid guide on / off |
| <kbd>Ctrl</kbd>+<kbd>G</kbd> | Switch the grid guide between white and black |
| <kbd>F</kbd> or <kbd>F11</kbd> | Cycle through the full-screen modes |
| <kbd>Esc</kbd> | Leave full screen |

## Horizon lock

These keys change **Roll angle correction** in **Stabilization**. They only work while **Lock
horizon** is on.

| Keys | Action |
|---|---|
| <kbd>E</kbd> / <kbd>R</kbd> | +0.5° / -0.5° |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> / <kbd>Ctrl</kbd>+<kbd>R</kbd> | +0.1° / -0.1° |
| <kbd>Alt</kbd>+<kbd>E</kbd> / <kbd>Alt</kbd>+<kbd>R</kbd> | +1° / -1° |
| <kbd>Shift</kbd>+<kbd>E</kbd> / <kbd>Shift</kbd>+<kbd>R</kbd> | +5° / -5° |

## Motion chart

| Keys | Action |
|---|---|
| <kbd>X</kbd>, <kbd>Y</kbd>, <kbd>Z</kbd>, <kbd>W</kbd> | Hide the first, second, third or fourth curve of the chart |
| <kbd>Shift</kbd>+<kbd>X</kbd>, <kbd>Shift</kbd>+<kbd>Y</kbd>, <kbd>Shift</kbd>+<kbd>Z</kbd>, <kbd>Shift</kbd>+<kbd>W</kbd> | Show that curve again |
| <kbd>Shift</kbd>+<kbd>G</kbd> | Chart shows the gyroscope |
| <kbd>Shift</kbd>+<kbd>A</kbd> | Chart shows the accelerometer |
| <kbd>Shift</kbd>+<kbd>M</kbd> | Chart shows the magnetometer |
| <kbd>Shift</kbd>+<kbd>Q</kbd> | Chart shows the orientation quaternions |

## Files, export and the render queue

| Keys | Action |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>S</kbd> | Save the project file (`.gyroflow`, including the motion data) |
| <kbd>Ctrl</kbd>+<kbd>W</kbd> | Export the current clip (or save the queued job you are editing) |
| <kbd>Ctrl</kbd>+<kbd>Q</kbd> | Add the current clip to the render queue |
| <kbd>Q</kbd> | Show / hide the render queue |
| <kbd>Ctrl</kbd>+<kbd>D</kbd> | Open the next file in the same folder |
| <kbd>Ctrl</kbd>+<kbd>A</kbd> | Open the previous file in the same folder |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> | While editing a queued job: save it and open the next job |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>A</kbd> | While editing a queued job: save it and open the previous job |

## Mouse controls on the timeline

| Mouse | Action |
|---|---|
| Scroll | Zoom the timeline in and out |
| <kbd>Alt</kbd>+scroll | Zoom the chart vertically |
| <kbd>Ctrl</kbd>+scroll (macOS: scroll sideways) | Pan along the timeline |
| Double-click | Reset the zoom |
| Right-click | Timeline menu: sync points, **Guess IMU orientation here**, **Estimate rolling shutter here**, **Estimate gyro bias here**, **Trim range** and **Chart display mode** |
| Click a sync point | Edit its offset |
| Double-click a sync point | Zoom in on it |
| Right-click a slider | **Reset value**, **Enable keyframing**, **Add keyframe** |
