---
layout: default
title: DJI O4 Gyrogate repair
description: >-
  Fix the DJI O4 Pro and O4 Lite "Gyrogate" jitter in Gyroflow-style stabilization: FPVFlow
  detects the false rotations recorded by the ICM-40609-D gyro and removes them from the motion
  data, with an optional corrected copy of the clip.
parent: "FPVFlow features"
nav_order: 3
---

# DJI O4 Gyrogate repair
{: .no_toc }

If your DJI O4 Pro or O4 Lite footage jerks and slowly sways after stabilizing it in Gyroflow,
even though the raw video looks fine, your air unit is probably affected by the DJI O4
"Gyrogate" problem. FPVFlow detects it when you open the clip and fixes the jitter by repairing
the motion data, without touching the video.

1. TOC
{:toc}

---

## What is DJI Gyrogate

DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use the TDK
ICM-40609-D gyro. Under motor vibration this gyro records short false rotations on a single
axis: up to about 2° in a few milliseconds. The camera then slowly pulls each one back over about
12 seconds.

The raw footage looks fine, because those rotations never happened. But the stabilization
trusts the motion data and faithfully "corrects" them, so the stabilized video jerks at each
false rotation and then sways while the pull-back plays out. The FPV community calls this
*DJI Gyrogate*.

The same gyro has a second, quieter symptom: a dense **single-axis tremor**, usually on roll.
Instead of a few large jumps, one axis carries many small spikes per second (around 20 to 50 °/s,
a few tenths of a degree each) while the other two stay calm. Each spike is too small to count as
a false rotation, but together they make the stabilized picture shiver.

## Is my air unit affected

| Unit | Gyro | Affected |
|:-----|:-----|:---------|
| O4 Lite (any production date) | ICM-40609-D | Yes |
| O4 Pro built in February 2026 or later | ICM-40609-D (most likely) | Yes |
| O4 Pro built in January 2026 | Either one: DJI was switching gyros | Maybe |
| O4 Pro built before 2026 | MP66 (original gyro) | No |

The clip does not name the gyro chip, so FPVFlow works it out from the model and from the
production date encoded in the serial number. You do not need to look anything up: the
**Motion data** panel shows the result. To decode a serial number yourself:

1. Skip the first 4 characters.
2. The 5th character is the year: `N` = 2025, `P` = 2026.
3. The 6th character is the month: `1` to `9` for January to September, then `A`, `B`, `C` for
   October, November and December.

For example, `9F2KP2...` was built in February 2026 (affected), and `9F2KN8...` in August 2025
(original MP66 gyro).

{: .note }
January 2026 (`9F2KP1...`) is a grey zone: both gyros were shipping that month. FPVFlow treats
those units as possibly affected and lets the clip itself decide.

## What FPVFlow shows when you open a clip

When you open a DJI O4 or O4 Pro clip, FPVFlow analyses its motion data right away. On an
affected clip the **Motion data** panel opens by itself and shows a warning with two lines: which
gyro the unit most likely has, and what was found in the clip.

The first line is one of these:

| Message | Meaning |
|:--------|:--------|
| DJI O4 Lite: ICM-40609-D gyro, which records false rotations under vibration (DJI Gyrogate). | Every O4 Lite is affected. |
| DJI O4 Pro built in MM/YYYY: most likely the new ICM-40609-D gyro, ... | O4 Pro from February 2026 on. |
| DJI O4 Pro built in MM/YYYY, when DJI was switching to the ICM-40609-D gyro: it may be affected by DJI Gyrogate. | January 2026 unit. |
| DJI O4 Pro built in MM/YYYY: original MP66 gyro, not affected by DJI Gyrogate. | Older O4 Pro. |
| DJI O4 Pro with an unknown production date: it may be affected by DJI Gyrogate. | The serial number could not be decoded. |

The second line tells you what the clip contains:

| Message | Meaning |
|:--------|:--------|
| N false rotation(s) found in this clip, X° in total, the largest Y°. | The clip has the defect. The repair is turned on. |
| Dense single-axis tremor in X s of the clip. | The clip has the tremor. It can follow the false rotations line. The repair is turned on. |
| No false rotations found in this clip. | Nothing to repair. |
| No 1 kHz attitude data in this clip. Record with RockSteady and HorizonSteady off to be able to repair it. | The clip lacks the data the repair needs. See [Requirements](#requirements). |
| N short single-axis jolt(s) found. With the original MP66 gyro these are usually real knocks, ... | Older O4 Pro: the repair stays off. See [Limits](#limits). |
| This is a corrected copy: its false rotations were already removed. | You opened a `_gyrofix` copy. |

## Repair the stabilization

The repair lives in the **Motion data** panel, under the detection message.

1. Open the affected clip in FPVFlow. **Remove DJI Gyrogate false rotations** is turned on
   automatically when false rotations are found (except on MP66 units). A project saved with an
   older version keeps the repair off: turn on **Remove DJI Gyrogate false rotations** yourself.
2. Play the stabilized preview, especially around the moments where it used to jerk.
3. Adjust **Sensitivity** if needed (see below).
4. Export as usual. The setting is saved with the project and with the render queue job.

For each false rotation, the repair removes the burst, compensates the camera's slow pull-back of
it, and smooths the result. Where the single-axis tremor piles up, it smooths that axis only
(low pass at 8 Hz), fading in and out over about half a second; the other axes and the calm parts
of the clip are not touched. Clips with neither are left untouched. The repair runs on
the motion data before the other motion data filters, such as **Glitch filtering** and the
**Low pass filter**.

You can turn the checkbox on yourself on any O4 or O4 Pro clip that has the 1 kHz attitude data,
or off if you want to compare.

## Sensitivity

**Sensitivity** sets how small a false rotation can be and still be removed. A rotation counts as
false when one axis jumps above a threshold for a few milliseconds while the other two axes stay
calm. The help icon next to the slider explains the same thing inside the app.

| Sensitivity | Threshold | What it catches |
|:------------|:----------|:----------------|
| 0% | 120 °/s | Only the large, obvious jolts. |
| 50% (default) | 60 °/s | Tuned against Betaflight blackbox logs. Suits most clips. |
| 100% | 30 °/s | Also small jolts, but it may mistake real knocks for false ones. |

The tremor floor follows the same setting: a spike counts towards the tremor above a quarter of
the threshold (15 °/s at 50%).

- **Raise it** if small jolts remain in the stabilized video with the repair on.
- **Lower it** if real motion, such as a knock or an impact, looks smoothed away.

The value is saved with the project and is also used for the corrected copy.

## Save a corrected copy

**Save corrected copy** writes a repaired version of the clip next to the original, named
`<name>_gyrofix.MP4` (for example `DJI_0001_gyrofix.MP4`).

- Only the attitude values inside the file are rewritten. The copy has the same size, and the
  video and audio are not changed.
- The original file is never modified.
- On APFS (the Mac's internal drive) the copy is a clone and takes almost no extra space.
- If a `_gyrofix` copy already exists, it is replaced.
- The copy opens like any DJI clip in FPVFlow, Gyroflow or the Gyroflow plugins, so you can use
  it outside FPVFlow too.

To make one:

1. Set **Sensitivity** to the value you want: the copy uses it.
2. Click **Save corrected copy** and wait until FPVFlow reports "Corrected copy saved to ...".
3. FPVFlow switches the motion data to the copy and keeps the rest of your project: sync, lens,
   audio and colour settings. **Remove DJI Gyrogate false rotations** is turned off for the copy,
   since its false rotations are already gone.

{: .note }
If neither false rotations nor tremor are found at the current sensitivity, no copy is written
and FPVFlow tells you so. The button is not available in the Android version.

## Requirements

- **RockSteady and HorizonSteady off.** The repair works on the 1 kHz attitude track that the O4
  only records with both off. Clips recorded with either one on show "No 1 kHz attitude data in
  this clip" and cannot be repaired.
- **The original DJI clip** (or a `_gyrofix` copy of it), loaded as the motion data. The repair
  works from the motion data alone; no blackbox log is needed.

## Limits

{: .important }
The repair removes most of the false motion, not all of it. The real fix has to come from a DJI
firmware update.

- **Older O4 Pro units with the MP66 gyro:** the repair stays off. FPVFlow may still report a few
  short single-axis jolts, but on these units they are usually real knocks, such as setting the
  drone down. You can still turn the repair on by hand.
- **Only when you open the clip in the app.** The repair is turned on automatically only when you
  open an affected clip in FPVFlow, not when you open a project saved with an older version.
  Files added straight to the render queue, or rendered from the command line, are not repaired unless their project has the repair on. If you batch-render O4
  footage, open each affected clip first, or render from `_gyrofix` copies.
- **Real fast motion.** At high sensitivity a real knock can be taken for a false rotation. Lower
  **Sensitivity** if that happens.
- **Real vibration and jello.** Jolts or jello that you can already see in the **raw** footage are
  real camera motion, not Gyrogate. The O4 only records the attitude after its own sensor fusion,
  and in those moments it is smoother than the picture, so no motion-data repair can remove them,
  and rolling shutter correction has nothing to work with. Look at the drone instead: bent or
  chipped props, worn motors, the camera mount, and prop wash in hard manoeuvres near obstacles.

## Credits

- The burst repair is a port of [o4fix](https://github.com/vadimbryl/o4fix) by vadimbryl, and
  with the tremor step off its output matches o4fix exactly on the public sample clips. The
  tremor step is FPVFlow's own: on the public O4 Pro sample it brings the roll jitter from 0.072
  to 0.017° per frame.
- Thanks to [DJIFix](https://github.com/parhipov/DJIFix) by parhipov and
  [djgyrofix](https://github.com/steamvogue/djgyrofix) by steamvogue for their analyses and sample
  footage.
- Background on the gyro change in the O4 Pro:
  [Oscar Liang's article](https://oscarliang.com/dji-o4-pro-gyro-stabilization-issue/).
