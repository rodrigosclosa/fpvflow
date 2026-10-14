---
layout: default
title: Smoothing algorithms
parent: Technical details
nav_order: 1
description: >-
  The smoothing algorithms in FPVFlow (No smoothing, Default, Plain 3D, Fixed camera), what every
  parameter does, horizon lock with automatic turn tilt, and how dynamic zooming and the zoom
  limit hide the borders.
---

# Smoothing algorithms
{: .no_toc }

The smoothing algorithm decides how calm the stabilized camera is. It takes the real camera
orientation, recorded by the gyro, and produces a smoothed orientation for a virtual camera. The
difference between the two is what FPVFlow rotates out of each frame.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## The idea in one paragraph

Think of the virtual camera as following the real one on a soft spring. The real camera shakes,
rolls and snaps through every stick input; the virtual camera follows the same overall path but
lags behind the quick movements. The softer the spring, the smoother the result and the bigger
the gap between the two cameras. A bigger gap means a bigger rotation to correct, and so a
bigger crop to hide the empty borders it leaves. Every smoothing setting is a trade between
smoothness and how much of the image you keep.

**Default** (with **Per axis** off) and **Plain 3D** smooth the 3D orientation as a whole
(quaternions), not three separate angles, so a fast roll does not leak into pitch or yaw. They
also run once forward and once backward through the clip, so the virtual camera does not trail
behind the real one in time. With **Per axis** on, **Default** smooths pitch, yaw and roll
separately instead.

## Choosing an algorithm

The algorithm is picked from the drop-down at the top of the **Stabilization** panel.

| Algorithm | What it does | Use it for |
|:----------|:-------------|:-----------|
| **Default** | Smoothing that adapts to how fast the camera rotates. | Almost everything, and every FPV clip as a starting point. |
| **Plain 3D** | Smoothing with one fixed strength for the whole clip. | Steady cruising where you want a predictable, uniform feel. |
| **Fixed camera** | The virtual camera points in one fixed direction. | Tripod-like shots from a hovering or handheld camera. |
| **No smoothing** | The virtual camera follows the real one exactly. | Correcting only the lens, the rolling shutter or the horizon. |

Most sliders can also change over the clip: right-click the slider, choose **Enable
keyframing**, then **Add keyframe** at each point of the timeline where you want a new value.

## Default

The default algorithm looks at how fast the camera is rotating at every moment and changes its
strength to match:

- When the camera moves slowly, it smooths heavily, so small shakes and drift disappear.
- When the camera rotates fast, such as a quick turn or a roll, it smooths lightly, so the
  virtual camera keeps up with the move instead of lagging far behind and forcing a huge crop.

It then runs a second pass over its own result. Where the first result stays close to the real
camera, there is room for more smoothing, and the second pass smooths harder. Where it already
drifts far from the real camera, the second pass keeps the lighter smoothing, so the zoom needed
for the fast parts of the clip stays under control.

The rotation speed that counts as "fast" also depends on the lens. A wide FPV lens shows the
same rotation as a smaller movement in the image than a narrow lens does, so with a wider lens
the algorithm tolerates faster rotation before it backs off.

| Parameter | Range | Default | What it changes |
|:----------|:------|:--------|:----------------|
| **Smoothness** | 0.1 to 100 % | 50 % | The overall strength. Higher values keep heavy smoothing on during faster movements. |
| **Pitch smoothness**, **Yaw smoothness**, **Roll smoothness** | 0.1 to 100 % | 50 % | Shown instead of **Smoothness** when **Per axis** is on. Each axis gets its own strength. |

Under **Advanced**:

| Parameter | Range | Default | What it changes |
|:----------|:------|:--------|:----------------|
| **Per axis** | on / off | off | Measures speed and applies smoothing per axis (pitch, yaw, roll) instead of for the rotation as a whole. |
| **Only within trim range** | on / off | on | Motion outside the trim range is ignored, so a crash or a hand grab before or after the part you keep does not affect the smoothing inside it. |
| **Max smoothness** | 0.1 to 5 s | 1 s | The time constant used when the camera is nearly still. Longer means a calmer camera during slow parts. |
| **Max smoothness at high velocity** | 0.01 to 1 s | 0.1 s | The time constant used during fast rotation. Shorter means the virtual camera follows fast moves more closely. |

{: .tip }
For freestyle, **Per axis** lets you keep yaw fairly loose, so turns still read as turns, while
smoothing roll and pitch harder. For cinematic cruising, raise **Smoothness** and keep **Lock
horizon** on.

## Plain 3D

Plain 3D applies the same smoothing everywhere in the clip. It is a classic exponential smoothing
of the orientation, run forward and backward.

| Parameter | Range | Default | What it changes |
|:----------|:------|:--------|:----------------|
| **Smoothness** | 0.01 to 10 s | 0.25 s | The time constant. Larger values give a smoother, slower virtual camera and need more zoom. |
| **Only within trim range** (Advanced) | on / off | on | Same as for Default. |

Because the strength never adapts, a value that looks good in a calm section can need a lot of
zoom during a fast move. The **Zoom limit** described below helps with that.

## Fixed camera

The virtual camera does not follow the real camera at all. It looks in the direction you set
with **Roll angle** (-180 to 180°), **Pitch angle** (-90 to 90°) and **Yaw angle** (-180 to
180°), all keyframeable. Any real movement away from that direction is rotated out, as far as
the image allows.

This only works when the real camera stays pointed roughly the same way, so it suits hovering
or handheld shots, not flying.

## No smoothing

The virtual camera is the real camera, so no shake is removed. The rest of the pipeline still
runs: lens undistortion, rolling shutter correction (see
[Rolling shutter correction]({{ '/technical/rolling-shutter.html' | relative_url }})) and horizon lock. Use it
when you want only one of those corrections and the original camera motion otherwise.

## Lock horizon

**Lock horizon** keeps the virtual camera level, whatever the drone does. It is applied before
the smoothing algorithm runs, so the smoothing settings also soften the horizon-locked motion.

| Parameter | Range | Default | What it changes |
|:----------|:------|:--------|:----------------|
| **Lock amount** | 0 to 100 % | 100 % | How much of the roll is removed. Below 100 % some of the original roll is kept. |
| **Roll angle correction** | -180 to 180° | 0° | An offset added to the locked horizon, to fix a camera mounted slightly crooked. |
| **Lock pitch angle** | on / off | off | Also locks the pitch (camera tilt up or down) to a fixed angle. |
| **Pitch angle correction** | -90 to 90° | 0° | The pitch to lock to, shown when **Lock pitch angle** is on. |
| **Use gravity vectors** | on / off | off | Only shown when the clip contains gravity vectors from the camera. Uses them to find the horizon. |
| **Integration method** | list | VQF | Only shown when the clip has both the camera's own orientation and raw IMU data, and **Motion data** uses the built-in orientation (**Integration method**: None). Chooses how the horizon is tracked. |

A level horizon needs motion data that knows where gravity is. If the horizon drifts or tilts
over the clip, follow the hint in the panel and try another integration method in the
**Motion data** panel.

### Automatic lock

**Automatic lock** is for flying. A fully locked horizon makes banked turns look strangely flat;
with automatic lock the horizon tilts into the turn the way a pilot expects, and comes back to
level when you stop rolling. Turning it on sets **Lock amount** to 100 % and disables that
slider.

FPVFlow measures the roll rate of the real camera. When it goes above the threshold, the horizon
is tilted by the roll rate times the multiplier (for example, 20°/s of roll with a multiplier of
1 tilts the horizon 20°).

| Parameter | Range | Default | What it changes |
|:----------|:------|:--------|:----------------|
| **Turn threshold** | 0 to 50°/s | 5°/s | Roll rates below this keep the horizon level. |
| **Turn smoothness** | 0 to 2000 ms | 500 ms | How smoothly the roll rate is measured and the tilt eases in and out. |
| **Turn multiplier** | 0 to 2x | 1x | How much tilt each degree per second of roll produces. |
| **Limit tilt acceleration** | on / off | off | Caps how fast the tilt itself can speed up or slow down. |
| **Tilt acceleration limit** | 0.1 to 30°/s² | 3°/s² | The cap, shown when **Limit tilt acceleration** is on. Lower values give gentler horizon moves. |

## Zooming and field of view

Rotating the frame leaves empty corners, and zooming in hides them. The zoom settings are below
the smoothing settings in the **Stabilization** panel.

After every change the panel shows **Max rotation** (the largest pitch, yaw and roll correction
in the clip) and **Max zoom**. Both grow with stronger smoothing.

### Zoom mode

| Mode | What it does |
|:-----|:-------------|
| **No zooming** | No crop at all. You see the empty borders wherever the correction needs them. |
| **Dynamic zooming** | The zoom follows what each moment needs: it moves in during big corrections and back out when the camera is calm. |
| **Static zoom** | One zoom for the whole clip, large enough for the worst moment. |

With **Dynamic zooming**, **Zooming speed** (0.1 to 15 s, default 4 s) sets how quickly the zoom
can change. Longer values give slow, unnoticeable zoom changes but stay zoomed in for longer
around a big correction.

Under **Advanced**, **Zooming method** chooses how the zoom curve is smoothed:

- **Envelope follower** (default) eases the zoom in and out but never zooms out further than
  the frame allows at that moment, so it does not uncover borders. FPVFlow recommends it, and
  switches to it, when the zooming speed is keyframed.
- **Gaussian filter** takes the largest zoom needed within the zooming speed window and blurs
  it over time. With a keyframed zooming speed it can show black borders.

### Zoom limit

**Zoom limit** (110 to 300 %, default 130 %) is shown for the Default and Plain 3D algorithms.
Instead of zooming as much as the smoothing demands, FPVFlow reduces the smoothing in the parts
of the clip that would need more zoom than the limit, and recalculates. The result is a slightly
less smooth camera in those moments and a crop that stays within the limit.

The limit is found by repeating this a few times. **Zoom limit iterations** under **Advanced**
(1 to 15, default 5) sets how many. More iterations give a more exact limit and take longer to
calculate.

{: .tip }
FPV footage often has a few violent moments, such as a crash, a snap roll or a prop strike, that
would need an extreme zoom. A zoom limit around the default keeps those moments from dictating
the crop of the whole clip.

### Lens correction and FOV

- **Lens correction** (0 to 100 %, default 100 %) sets how much of the lens distortion is
  removed. Lower values keep part of the original fisheye look.
- **FOV** under **Advanced** (0.1 to 3, default 1.0) scales the output field of view on top of
  the automatic zoom. Values above 1.0 show more of the image but can show black borders, and
  the panel warns about it.
- **Additional 3D rotation** under **Advanced** adds a fixed pitch, yaw and roll to the virtual
  camera, for example to reframe a shot or to frame a vertical video.

### Stabilize focal length

**Stabilize focal length** is only shown for clips that record a focal length for every frame,
as some cameras with electronic zoom lenses do. It smooths small jumps in the recorded focal
length; **Smoothness** (default 30 %) sets how much.

## Video speed

**Video speed** under **Advanced** changes the playback speed of the clip. Next to the slider are
three small check boxes, **Link with smoothing**, **Link with zooming speed** and **Link with
zooming limit**, all on by default. They scale the smoothing, the zooming speed and the zoom
limit with the speed, so a sped-up or slowed-down clip keeps a similar look.
