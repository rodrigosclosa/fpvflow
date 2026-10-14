---
layout: default
title: The .gcsv format
parent: Motion data logging
nav_order: 4
description: >-
  Reference for the Gyroflow .gcsv motion log format read by FPVFlow: first line, header keys,
  scales and units, data columns, IMU orientation and an example file for a DIY gyro logger.
---

# The .gcsv format
{: .no_toc }

`.gcsv` is a plain text format for gyro logs, defined by the Gyroflow project. If you build your
own logger, or convert data from a device FPVFlow does not read, writing a `.gcsv` file is the
simplest way in. This page describes what FPVFlow actually reads from it.

1. TOC
{:toc}

---

## Layout

A `.gcsv` file has three parts, all comma-separated text:

1. **An identification line**, exactly `GYROFLOW IMU LOG` or `CAMERA IMU LOG`, as the very first
   bytes of the file (no blank line or byte order mark before it).
2. **Header lines**, one `key,value` pair per line.
3. **A column line** that starts with `t` (or `time`), followed by one data row per sample.

FPVFlow recognises the file by its first line, not by the extension, so `.gcsv`, `.csv`, `.txt` and
`.bin` all work. Use `.gcsv` anyway: it is the extension FPVFlow looks for when it searches for a log
next to a video. Spaces around values are trimmed.

## Header keys

Every key is optional. Other keys are kept as extra metadata. Two of them change how FPVFlow
works: `has_accurate_timestamps` (`true` or `1`) marks the timestamps as exact, and
`additional_sync_points` (times in milliseconds, separated by `;`) adds sync points used by
**Synchronization**. FPVFlow writes the second one itself when you merge several clips with
**Merge them into one video**.

| Key | Meaning | Default |
|:----|:--------|:--------|
| `tscale` | Seconds per unit of the time column. `0.001` means the time is in milliseconds. | `0.001` |
| `gscale` | Multiply a raw gyro value by this to get radians per second. | `1` |
| `ascale` | Multiply a raw accelerometer value by this to get g (1 g = 9.80665 m/s²). | `1` |
| `mscale` | Scale of the optional magnetometer columns. | `1` |
| `orientation` | [IMU orientation]({{ '/logging/orientation.html' | relative_url }}) string, such as `YxZ`. | `xzY` |
| `id` | Name of the logger or camera model. Underscores are shown as spaces. | `NoID` |
| `vendor` | Maker of the logger. Shown in **Detected format** together with `id`. | `gcsv` |
| `lensprofile` | Lens profile to load automatically: the identifier of a profile in the lens database, or its file name. | none |
| `frame_readout_time` | Rolling shutter readout time of the camera, in milliseconds. | none |
| `frame_readout_direction` | Readout direction: `0` or `TopToBottom`, `1` or `BottomToTop`, `2` or `LeftToRight`, `3` or `RightToLeft`. Used only together with `frame_readout_time`. | top to bottom |

{: .note }
A `version` line is common in `.gcsv` files written by other tools. FPVFlow accepts it but does not
use it.

{: .note }
`lensprofile`, `frame_readout_time` and the other clip settings are applied when the log is loaded
together with the video (same name, same folder), or with **Load all metadata** checked in
**Motion data**. A log opened on its own with **Open file** only provides the motion.

{: .tip }
Keep the header short. `id`, `vendor` and the readout settings are read from the first 500 bytes of
the file.

## Data rows

After the column line, each row is one sample. Columns are read **by position**; the names in the
column line are only for people reading the file.

| Position | Content |
|:---------|:--------|
| 1 | Time, in units of `tscale` |
| 2-4 | Gyro X, Y, Z (raw, multiplied by `gscale`) |
| 5-7 | Accelerometer X, Y, Z (raw, multiplied by `ascale`), optional |
| 8-10 | Magnetometer X, Y, Z (raw, multiplied by `mscale`), optional |

A row with only the time and the gyro is valid. The accelerometer is read when a row has at least
seven values, the magnetometer when it has at least ten.

Rules worth knowing:

- **Time does not have to start at zero.** FPVFlow shifts the log so that its first sample is time
  zero. The offset to the video is found during synchronization anyway.
- **Keep time increasing.** Rows whose time cannot be read as a number are skipped.
- **The sample rate is free.** FPVFlow works with the timestamps, so slightly uneven intervals are
  fine. A steady rate is still better for sync quality.
- **The accelerometer can be slower than the gyro.** Write it only on the rows where you have a new
  reading; rows with only time and gyro are valid. FPVFlow interpolates the accelerometer to the
  gyro timestamps.

## Example

A logger with a 16-bit gyro at ±2000 °/s and an accelerometer at ±16 g, logging time in
milliseconds:

```
GYROFLOW IMU LOG
id,diy_logger
vendor,MyLogger
orientation,YxZ
tscale,0.001
gscale,0.00106526
ascale,0.00048828
t,gx,gy,gz,ax,ay,az
0,12,-5,3,10,2048,-15
1,15,-6,2,11,2047,-14
2,19,-4,1,9,2049,-15
```

Where the scales come from:

- Gyro: ±2000 °/s over a signed 16-bit range is 2000 / 32768 = 0.061 °/s per unit, which is
  0.061 × π / 180 ≈ 0.00106526 rad/s per unit.
- Accelerometer: ±16 g over the same range is 16 / 32768 ≈ 0.00048828 g per unit.

Take the real sensitivity from your IMU datasheet. If your logger already converts the values, set
the scale to match: data in rad/s needs `gscale,1`, data in °/s needs `gscale,0.0174533`.

## Using a .gcsv file

- **With a video:** name it like the video (`clip.mp4` and `clip.gcsv` in the same folder) and
  FPVFlow loads it when you open the video. This works for MP4, MOV and MKV videos that carry no
  motion data of their own. Otherwise use **Open file** in **Motion data**.
- **With an image sequence** (DNG, EXR, JPEG or PNG): FPVFlow looks for a `.gcsv` with the same
  name as the frame (`0000.gcsv` for `0000.dng`), or with the sequence name without the frame
  number (`clip.gcsv` for `clip_00001.dng`).

After loading, check **Detected format** in **Motion data**: it shows the `vendor` and `id` from your
header, which confirms FPVFlow read the file as `.gcsv`. Then check the orientation and sync as for
any external log; see [Motion data logging]({{ '/logging/' | relative_url }}).
