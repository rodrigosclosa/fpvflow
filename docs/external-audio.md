---
layout: default
title: External audio
description: >-
  Add a separately recorded audio track (DJI Mic, phone, recorder) to your FPV footage in FPVFlow,
  sync it automatically from the arming beep and the disarm, check it in the preview and export it
  without losing quality.
parent: "FPVFlow features"
nav_order: 2
---

# External audio
{: .no_toc }

FPV setups often record sound separately: a DJI Mic on the pilot, a recorder on the field, or a
naked GoPro with no microphone at all. FPVFlow imports that track, lines it up with the video and
embeds it in the stabilized export, so you do not need a second pass through an editor.

1. TOC
{:toc}

---

## What it does

- Imports a separate audio file and keeps it with the clip.
- Finds the offset automatically, or lets you set it by hand.
- Shows the waveform on the timeline, below the gyro chart.
- Plays the track in the preview, aligned with the video, so you can check the sync by ear.
- Embeds the track in the exported video, keeping its original PCM format (WAV) where the container
  allows.
- Keeps the track, its offset and its format setting per clip in the render queue.

The **External audio** panel is in the left column, below **Motion data**. It is enabled once a
video is loaded.

## Supported files

The file picker accepts `.wav`, `.m4a`, `.mp3`, `.flac`, `.aac` and `.mp4`.

WAV covers integer PCM (8, 16, 24 and 32-bit) and 32-bit float, the format the DJI Mic records in.
After import the panel shows what was detected, for example `48000 Hz, 2 channels, 32-bit float`,
followed by the file path. Check that line: it is what decides how the audio is exported.

## Import a track

1. Load your video and its motion data as usual.
2. In the **External audio** panel, click **Import external audio** and pick the file.
3. Wait for the panel to finish decoding. The detected format appears under the button and the
   waveform appears on the timeline.

To use a different file, click **Replace audio file**. To drop the track, click **Remove audio**.

{: .note }
A track belongs to the clip it was aligned with. Loading another video removes it. Opening a saved
`.gyroflow` project brings it back, with its offset.

## Sync automatically

Click **Auto-sync audio**. The button needs motion data, so it stays disabled until the gyro is
loaded. The panel locks while the analysis runs, then moves the **Offset** to the result and shows a
line such as:

> Confidence: 95% — aligned by start of movement

The result is a starting point: you can still adjust the offset by hand afterwards.

### How it finds the offset

Which method runs depends on the motion data of the clip. The result line tells you which one ran:
"aligned by start of movement" or "aligned by propeller vibration".

**Motion data that is orientation-only or sampled below 300 Hz (DJI O4 Pro and similar).** Such
data cannot carry propeller vibration: an O4 Pro, for example, delivers its orientation about twice a
second. Instead the auto-sync anchors on sounds the drone makes at precise moments, which it can
find even with a crowd or music around. It tries them in this order and stops at the first that holds:

1. **Arming beep and disarm.** The three-tone beep the drone plays when it arms is found in the
   audio. For a camera set to record on arm, the first frame follows the beep by a fixed delay. The
   propellers stopping marks the last frame of a recording that stops on disarm. Both ends must
   agree before the result is accepted.
2. **Disarm only.** If the recording did not start on arm but still stopped on disarm, the
   propellers stopping anchors the end, as long as the arming beep then falls inside the clip and
   the ground rest (below), when found, agrees with it.
3. **Ground rest, for recordings started by hand.** When you start recording before arming, the clip
   often shows the drone being carried into place. The sync uses the moment the drone leaves its
   rest on the ground, where the arming beep also happens. If the rest disagrees with the disarm,
   the rest wins, which catches a recording stopped by hand mid-flight.
4. **Older fallbacks.** When none of these is available, it pairs the take-off heard in the audio
   with the lift-off seen in the motion data, and if that fails too it correlates the two signals.

The panel reports the result of all of these as "aligned by start of movement". The three anchors
always report a fixed confidence:

| Confidence | Anchor used |
|:--|:--|
| 95% | Arming beep and disarm agree |
| 70% | Disarm only |
| 60% | Ground rest |
| Any other value | Older fallbacks: check by hand |

The older fallbacks can report any confidence, including a high one, and still be off, so a value
other than 95%, 70% or 60% always deserves a check by hand.

On hand-aligned test clips the result lands within 0.15 s for recordings that run from arm to
disarm, and within about 0.5 s (1.35 s at worst) for recordings started by hand. The older fallbacks
can be off by a second or more.

{: .warning }
A recording that is both started and stopped by hand and has no arming beep on the audio falls back
to the older methods. Always check that result by hand.

**Raw gyro data sampled at 300 Hz or more.** Here the auto-sync correlates the propeller
vibration picked up by the microphone with the vibration read by the gyro, and the result line says
"aligned by propeller vibration". The **Advanced** section of the panel tunes this method
only:

- **Detect propeller band automatically** (on by default) finds the blade frequency band in the
  signal. Turn it off to set the **Band** yourself (150 - 900 Hz by default).
- **Gyro high-pass** (30 Hz by default) removes intentional movement from the gyro, leaving only
  vibration.

### When the confidence is low

Below 30% the result line turns orange and reads "weak match, check manually". With the vibration
method this almost always means the blade vibration did not reach the gyro (a gimbal isolating it)
or the microphone was far from the drone. "Not enough data to sync" means there was no usable motion
data.

In either case, set the offset by hand as described below.

## Set the offset by hand

The **Offset** slider is in milliseconds and covers -60 to +60 seconds. Under it, FPVFlow shows the
same value in frames at the clip's frame rate, which is how you would compare it against an editor's
timeline.

- **Drag the waveform** on the timeline left or right until it lines up with the gyro trace above
  it. This is the quickest way to get close. The drag follows the timeline zoom, so zoom in for fine
  moves.
- **Type a value** in the field next to the slider for an exact offset.
- Right-click the slider and choose **Reset value** to go back to 0.

A positive offset means the microphone started recording before the camera.

{: .tip }
The arming beep makes a clean visual mark on the waveform. Line it up with the moment the drone arms
in the video, then confirm by ear in the preview.

To hide the waveform, toggle the **♪** button on the left edge of the timeline (tooltip **Audio
waveform**). It only appears while a track is loaded.

## Check it in the preview

Press play. The external track plays in the preview, aligned with the video. After you change the
offset, the next play takes a moment while FPVFlow prepares the
aligned track.

## Export

The external track replaces the camera's audio in the exported file. **Export audio** must be on in
**Export settings**; it is not available when you change the video speed.

### Format preservation

**Preserve original audio format** is on by default. It keeps the original bit depth and sample
rate: 32-bit float stays 32-bit float, with no silent conversion. A badge above the checkbox tells
you, before you export, what will happen with the output container you picked:

| Detected format | Exported as | Containers that keep it |
|:--|:--|:--|
| 32-bit float | PCM 32-bit float | `.mov`, `.mkv` |
| 24-bit int, 32-bit int | PCM 24-bit | `.mov`, `.mkv` |
| 16-bit int, 8-bit | PCM 16-bit | `.mov`, `.mkv` |
| compressed (AAC, MP3, FLAC) | AAC | `.mp4`, `.mov`, `.mkv` |

{: .note }
FLAC is lossless, but FPVFlow treats it as compressed and always exports it as AAC. If you need
lossless audio in the export, convert the FLAC file to WAV before importing it.

- A green badge reads "Audio: ... preserved" and names the codec.
- A red badge means the source does not fit the container, for example 32-bit float in `.mp4`. In
  FPVFlow the container follows the video codec, so pick **ProRes**, **DNxHD** or **CineForm** to get
  a `.mov`.

{: .important }
If you export anyway with a red badge, the audio is converted to AAC. To accept that on purpose, turn
off **Preserve original audio format**; the badge then reads "Audio: will be converted to AAC".

## Render queue and projects

Each clip in the render queue keeps its own external track, offset and format setting, together
with its LUT and colour adjustments. They are restored when you reopen the app or edit a queued job.

The track is stored by its file path, both in the queue and in `.gyroflow` projects, and is read
again at render time.

{: .warning }
Do not move or rename the audio file until the clips that use it have finished rendering.

## Recording tips

- **Do not start the microphone too early.** The **Offset** slider, the field and dragging the
  waveform only go to ±60 seconds, so start the microphone less than a minute before the camera to
  be able to fine-tune the result.
- **Record 32-bit float and export to `.mov` or `.mkv`** to keep the full headroom of a DJI Mic
  recording.

For motion data that is orientation-only or sampled below 300 Hz (the beep and disarm method):

- **Start the microphone before you arm.** The arming beep is the strongest anchor, and the
  auto-sync looks for it in the first two minutes of the audio.
- **Let the camera record from arm to disarm** when you can. That gives the auto-sync both ends of
  the flight and the best accuracy.
- **Keep the microphone close to the drone at arming.** A beep or a propeller stop that does not
  stand out from the background cannot be used.

For the gyro synchronization itself (video to motion data), see the
[Gyroflow documentation](https://docs.gyroflow.xyz). External audio sync is a separate step and does
not change it.
