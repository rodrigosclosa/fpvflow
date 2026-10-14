---
layout: default
title: Gyro filtering
parent: Technical details
nav_order: 3
description: >-
  Clean up FPV gyro data in FPVFlow: what makes motion data good or bad, the low pass, median and
  glitch filters, and the DJI O4 Gyrogate repair that removes false rotations from O4 Pro and O4
  Lite clips.
---

# Gyro filtering
{: .no_toc }

Stabilization is only as good as the motion data behind it. A smooth, well-synced gyro track
gives a locked-in result; noise, glitches or false rotations in the data turn straight into
jitter in the stabilized video. This page covers what makes motion data good and the filters
FPVFlow offers to clean it.
{: .fs-5 .fw-300 }

1. TOC
{:toc}

---

## What good motion data looks like

### Enough samples, recorded cleanly

The camera motion you want to remove is fairly slow compared with what a gyro can measure, so
the logging rate does not need to be extreme. What matters more is that each sample is clean.

The main way a sample goes bad is aliasing. A gyro on a quad also picks up motor and prop
vibration at hundreds of hertz. If the logger records slower than twice that vibration
frequency without filtering it out first, the vibration does not disappear: it folds down into
the low frequencies and looks exactly like real camera shake. No filter applied afterwards can
tell the two apart. So the filtering has to happen before the data is logged, in the gyro chip
or the flight controller. With a camera's built-in gyro, that part is up to the camera maker.

### Gyro and camera moving together

The gyro has to feel the same motion as the camera. With a camera's built-in gyro this is a
given. With Betaflight blackbox data it is not: if the flight controller is soft-mounted and the
camera is hard-mounted, or the other way round, the two move differently under vibration and in
hard manoeuvres, and the stabilization follows the flight controller instead of the image. Mount
them so they move as one body, and keep any play out of the camera mount.

## The filters in the Motion data panel

FPVFlow has three general filters in the **Motion data** panel. Each one is off by default.

Which filters apply depends on the **Integration method** in the same panel. With **None**, the
stabilization uses the orientation the camera recorded itself (the tooltip reads "Use built-in
quaternions instead of IMU data"). With any other method, FPVFlow builds the orientation from the
raw gyro and accelerometer samples.

| Filter | Raw gyro and accelerometer | Camera's built-in orientation |
|:-------|:--------------------------:|:-----------------------------:|
| **Low pass filter** | Yes | Yes |
| **Median filter** | Yes | No |
| **Glitch filtering** | No | Yes |
| **Remove DJI Gyrogate false rotations** | No | Yes |

All the filters run forward and backward over the clip, so they do not shift the motion in time
and do not upset the sync.

### Low pass filter

Removes motion faster than the cutoff frequency. The value is in hertz (default 50 Hz when you
turn it on), and a lower cutoff means more filtering. It is a second-order Butterworth filter
applied in both directions.

Use it when the stabilized video shows fine, fast jitter that the raw footage does not have,
which usually means high-frequency noise in the gyro data. Do not go too low: real fast camera
motion also gets filtered, and the stabilization stops correcting it.

### Median filter

Replaces each sample with the median of its neighbours; the value is the window size in samples
(default 5). A median removes isolated spikes, single bad samples far from their neighbours,
without blurring the motion around them the way a low pass does. It only works on raw gyro and
accelerometer data.

### Glitch filtering

Some cameras now and then record a short burst of corrupt orientation data: the rotation rate
swings wildly and rings for a moment, unlike real motion, which follows a trend even when it is
fast. A low pass would just smear such a burst over its neighbours.

**Glitch filtering** detects these bursts and replaces them with a smooth path between the last
good sample before and the first good sample after. It compares each sample with the local trend
and flags the ones that stand out far above the clip's own normal level, then repeats the search
so that smaller glitches hidden next to big ones are also caught.

**Strength** (0 to 100 %, default 50 %) sets how aggressive the detection is. Higher values catch
weaker and longer bursts and search more times, but are more likely to touch real fast motion.
Lower values only fix the large, obvious glitches. Regardless of strength, an event that lasts
longer than about 0.75 to 2.25 seconds (depending on the strength) is treated as possible real
motion and left alone.

{: .note }
**Glitch filtering** is not in Gyroflow 1.6.3; FPVFlow takes it from newer Gyroflow development
code. Gyroflow 1.6.3 and video editor plugins built from it do not apply it.

### The low pass filter in Synchronization

The **Synchronization** panel has its own **Low pass filter**. It only smooths the data used to
find the sync offset, and does not change the motion used for stabilization.

## DJI O4 Gyrogate repair

{: .note }
This repair is an FPVFlow feature; it is not in Gyroflow. For the full user guide, see
[DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }}).

DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use a gyro chip
(TDK ICM-40609-D) that records short false rotations under motor vibration: one axis jumps for a
few milliseconds while the other two stay calm. The camera's own orientation tracking then slowly
pulls each false jump back over roughly 12 seconds. The raw video is fine, but the stabilization
believes the motion data and "corrects" movements that never happened, so the result jerks and
then slowly sways.

None of the general filters can fix this. The false jumps are too large and too fast for a low
pass, and the slow pull-back that follows looks like real, gentle motion. FPVFlow therefore has a
dedicated repair, **Remove DJI Gyrogate false rotations**, in the **Motion data** panel:

1. It finds the single-axis bursts in the camera's orientation track.
2. It removes them.
3. It adds back the slow pull-back that the camera applied to each removed burst, so the
   recovery drift disappears too.
4. It applies a gentle low pass to the result, stronger in the stretches where a lot was removed.

The repair runs before glitch filtering and the low pass filter, so they never see the raw
bursts. A clip in which no false rotation is found is left completely untouched.

**Sensitivity** sets how small a jump can be and still count as false:

| Sensitivity | Threshold | Effect |
|:------------|:----------|:-------|
| 0 % | 120°/s | Only the largest, most obvious jolts. |
| 50 % (default) | 60°/s | Tuned against Betaflight blackbox logs. |
| 100 % | 30°/s | Also small jolts, with a higher risk of removing a real knock. |

FPVFlow checks every DJI O4 clip when it opens, reports the likely gyro chip and the number of
false rotations, and turns the repair on for affected clips. **Save corrected copy** writes a
copy of the clip with the repaired orientation (`<name>_gyrofix.MP4`, video and audio
unchanged), which other tools, including Gyroflow and the
[video editor plugins]({{ '/integrations.html' | relative_url }}), can then use.

{: .warning }
The repair needs the 1 kHz orientation track that the O4 only records with **RockSteady and
HorizonSteady turned off**. Clips recorded with in-camera stabilization on cannot be repaired.
