---
layout: default
title: FAQ and troubleshooting
description: "Answers to common FPVFlow questions: supported cameras, DJI O4 Gyrogate jerks, RockSteady, external audio that does not sync, the corrected copy, bug reports and translations."
nav_order: 11
---

# FAQ and troubleshooting
{: .no_toc }

1. TOC
{:toc}

---

## Which cameras does FPVFlow work with?

Every camera and gyro source that Gyroflow 1.6.3 supports. For FPV that includes:

- **DJI:** O3 and O4 Air Unit, Avata, Avata 2, Action 2/4/5/6, Neo, Neo 2
- **GoPro:** HERO 5 and later, including clips recorded with HyperSmooth on (HERO 8 and later)
- **Insta360:** Go, GO 2, GO 3, Ace, Ace Pro, One R and others, plus Caddx Peanut
- **RunCam:** Thumb series, RunCam 5 Orange and others through their CSV logs
- **Cameras without a gyro:** use a **Betaflight blackbox** log (`.bfl`, `.bbl`, `.csv`) from the flight controller

See the full list of supported sources in the [FPVFlow README](https://github.com/rodrigosclosa/fpvflow#supported-gyro-sources) and the [Gyroflow documentation](https://docs.gyroflow.xyz) for camera settings and the blackbox workflow.

The [Gyrogate repair]({{ '/gyrogate.html' | relative_url }}) applies only to DJI O4 footage. [Colour grading]({{ '/colour-grading.html' | relative_url }}) and [external audio]({{ '/external-audio.html' | relative_url }}) work with any clip.

## Why does my DJI O4 clip jerk or sway after stabilization?

Most likely it is the DJI O4 "Gyrogate" defect. DJI O4 Air Unit Pro units built since about February 2026, and every O4 Lite, use the TDK ICM-40609-D gyro. Under motor vibration it records short false rotations on one axis, up to about 2° in a few milliseconds, and the camera then pulls each one back over about 12 seconds. The raw video looks fine, but the stabilization faithfully corrects rotations that never happened.

FPVFlow handles it for you:

1. Load the clip. On an affected clip the **Motion data** panel opens by itself and says which gyro your unit most likely has and how many false rotations the clip contains.
2. **Remove DJI Gyrogate false rotations** is already turned on for affected clips. Check the preview.
3. If small jolts remain, raise the **Sensitivity** slider. If a real knock or impact looks smoothed away, lower it. 50% is the default and suits most clips.

{: .note }
The repair removes most of the false motion, not all of it. The real fix has to come from a DJI firmware update.

On O4 Pro units with the original MP66 gyro the repair stays off by default: the short jolts found there are usually real knocks, such as setting the drone down. If the clip is not affected and still looks wrong, check the lens profile and the synchronization; the [Gyroflow documentation](https://docs.gyroflow.xyz) covers both.

More details on the [Gyrogate repair page]({{ '/gyrogate.html' | relative_url }}).

## Why must RockSteady be off?

The Gyrogate repair needs the 1 kHz attitude track that the O4 records only when **RockSteady and HorizonSteady are off**. If they were on, the **Motion data** panel says "No 1 kHz attitude data in this clip. Record with RockSteady and HorizonSteady off to be able to repair it.", and the repair and the corrected copy are not available.

Turn both off before you fly, then stabilize in FPVFlow. For recommended camera settings in general, see the [Gyroflow documentation](https://docs.gyroflow.xyz).

## Why is my external audio not syncing?

**Auto-sync audio** needs motion data, so the button is disabled until the clip's gyro data is loaded. How it aligns the audio depends on the motion data, and the line under the button tells you which method it used.

**"aligned by propeller vibration"**: the clip has a raw gyro sampled at 300 Hz or more, fast enough to carry the vibration of the propellers. FPVFlow matches the vibration heard by the microphone with the vibration read by the gyro. A weak match there almost always means:

- The vibration did not reach the gyro, for example because a gimbal isolates it.
- The microphone was too far from the drone.

**"aligned by start of movement"**: DJI O4 footage and other clips whose motion data holds only the orientation, or a gyro sampled below 300 Hz, carry no propeller vibration. FPVFlow then anchors on sounds the drone makes at precise moments:

- **The arming beep.** For a camera set to record on arm, the first frame follows the beep by a fixed delay.
- **The disarm.** The propellers stopping marks the last frame of a recording that stops on disarm.
- **Recordings started by hand.** The moment the drone leaves its rest on the ground is used, cross-checked against the disarm.

When none of these is available, it falls back to less accurate methods (pairing the take-off heard in the audio with the lift-off seen in the motion data, then correlating the two signals), which can be off by a second or more. Common reasons:

- The audio recorder was started after arming, so the beep is not in the track.
- The microphone was too far from the drone to pick up the beep or the propellers.
- The recording was both started and stopped by hand, with no arming beep.

"weak match, check manually" or "Not enough data to sync" on the result line means you should set the offset yourself:

1. Play the clip. The external audio plays in the preview, aligned with the video.
2. Adjust the **Offset** value until the sound matches the picture. The waveform on the timeline, below the gyro chart, helps you line up sharp sounds.
3. When you add the clip to the render queue, its audio file and offset are kept with it.

{: .tip }
Recording with the camera set to record on arm and disarm, and starting the audio before you arm, gives auto-sync the best chance: on hand-aligned test clips it lands within 0.15 s for recordings that run from arm to disarm.

If the audio is missing from the exported file, check that **Export audio** is on in the **Export settings** panel. Audio export is not available when you change the video speed.

## Where is the corrected copy?

**Save corrected copy** in the **Motion data** panel writes `<name>_gyrofix.MP4` in the same folder as the original clip. Only the attitude data is rewritten: the file has the same size, the video and audio are untouched, and the original is never modified.

After saving, FPVFlow uses the copy as the motion data and keeps your sync, lens, audio and colour settings. The copy opens like any DJI clip in FPVFlow, Gyroflow or the Gyroflow video editor plugins. On a Mac's internal APFS drive it is a clone and takes almost no extra space.

If no copy appears:

- "No false rotations were found at this sensitivity, so no copy was written." means there was nothing to fix. Raise the **Sensitivity** if you still see jolts.
- The button is unavailable when the clip has no 1 kHz attitude data (RockSteady or HorizonSteady was on) or when the clip is already a corrected copy.
- "Could not save the corrected copy" is followed by the reason. If the reason mentions permissions or a read-only file system, copy the clip to your disk first. If it says the clip is not an original DJI O4 recording or its metadata is corrupt, the clip cannot be repaired.

## How do I report a bug?

Open an issue in the [FPVFlow issue tracker](https://github.com/rodrigosclosa/fpvflow/issues). Include your operating system, the FPVFlow version shown in the app, what you did and what happened. A short sample clip, with its audio file if the problem is about external audio, makes a bug much easier to fix.

{: .important }
Please do not take FPVFlow problems to the Gyroflow project. Its Discord and issue tracker are for the official application, and its maintainers did not build this fork and cannot support it.

## What is the difference between FPVFlow and Gyroflow?

FPVFlow is a fork of Gyroflow 1.6.3. The stabilization engine, the lens profile database and the `.gyroflow` project format are Gyroflow's, unchanged. FPVFlow adds three features that do not exist upstream:

- The [DJI O4 Gyrogate repair]({{ '/gyrogate.html' | relative_url }})
- [Colour grading with `.cube` LUTs]({{ '/colour-grading.html' | relative_url }}) applied during the export
- An [external audio track]({{ '/external-audio.html' | relative_url }}) with automatic synchronization

FPVFlow numbers its own versions from 1.0.0. To see which app you are running, check the version in the app: FPVFlow shows "v1.0.x · baseado no Gyroflow 1.6.3". Both apps can be installed side by side and share lens profiles and preferences.

FPVFlow is built for Windows, macOS and Linux only. On Android and iOS, use Gyroflow.

## Can I use FPVFlow in my language?

Yes. Choose it under **Language** in the **Advanced** panel. Gyroflow's interface is translated into more than 20 languages, and those translations come with FPVFlow.

Gyroflow's translations are managed upstream on [Crowdin](https://crowdin.com/project/gyroflow). The strings added by FPVFlow (the colour panel, the external audio panel and the Gyrogate section of the motion data panel) are not in that project, so translating them there would not reach FPVFlow. If you want to translate them, [open an issue](https://github.com/rodrigosclosa/fpvflow/issues) in the FPVFlow repository.
