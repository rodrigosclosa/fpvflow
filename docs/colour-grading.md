---
layout: default
title: Colour grading
description: >-
  Apply a .cube LUT and 13 colour adjustments while FPVFlow stabilizes, so the exported FPV
  clip is already graded, keeps its bit depth and is tagged Rec.709.
parent: "FPVFlow features"
nav_order: 1
---

# Colour grading
{: .no_toc }

Cameras that record in a log profile (DJI D-Log M, GoPro Flat and similar) look flat and grey
until a conversion LUT turns them into normal Rec.709 video. In plain Gyroflow that means
rendering twice: once to stabilize, once more in an editor to grade.

FPVFlow applies the LUT **during stabilization**, in the **Color** panel, so the exported file is
already graded. On top of the LUT there are 13 colour adjustments, which also work without any
LUT loaded.

1. TOC
{:toc}

---

## Where the Color panel is

The **Color** panel sits in the right-hand column, between **Stabilization** and
**Export settings**. It is greyed out until a video is loaded.

Everything you set here shows up live in the preview and is applied to the export.

## Load a LUT

1. Load your video as usual.
2. In the **Color** panel, click **Load LUT**.
3. Pick a `.cube` file.

The panel then shows the LUT title (when the file has one) and its size, for example
`DJI D-Log M to Rec.709 - 33³ entries`, with the file path underneath. The button changes to
**Replace LUT**, which swaps the table for another one.

To go back to the original image, click **Remove LUT**.

{: .important }
A LUT is built for one specific input. A D-Log M to Rec.709 LUT only gives correct colours on
footage recorded in D-Log M. The `.cube` format does not say what input it expects, so FPVFlow
cannot check this for you: use the LUT your camera maker publishes for the profile you recorded in.

### Supported files

- **Format:** Adobe Cube (`.cube` or `.CUBE`), the format camera makers ship their conversion
  LUTs in.
- **Size:** 3D tables up to 256 entries per side. The usual sizes are 17, 33 and 65. Tables larger
  than 65 are downsampled to 65 for processing.
- **Domain:** `DOMAIN_MIN` and `DOMAIN_MAX` are respected.
- **Interpolation:** trilinear, computed in floating point.

{: .warning }
The parser also reads 1D tables (`LUT_1D_SIZE`), but the GPU path, which the preview and
exports normally use, only applies 3D tables. Use a 3D LUT. In the same way, **Nitidez**
(sharpness) only applies on the GPU path; the CPU fallback skips it.

### When a LUT does not load

If a file cannot be read, the panel shows the reason in red. The message names the problem and,
where it applies, the line number:

- *"Expected ... entries, found ... - the file looks truncated"*: the download was cut short.
  Download the LUT again.
- *"Line ... is not three numbers"*: a data line is damaged.
- *"No LUT_1D_SIZE or LUT_3D_SIZE in the file"*: the file is not a valid `.cube` LUT.

## Build a LUT library

If you use the same few LUTs all the time, point FPVFlow at the folder that holds them once,
instead of browsing for a file on every clip.

1. Click **Set LUT folder** and choose the folder with your `.cube` files.
2. A **Library** dropdown appears above the buttons, listing every `.cube` file in that folder,
   sorted by name.
3. Pick a LUT from the dropdown to apply it. **Choose a LUT...** at the top of the list is only
   a placeholder.

The folder is remembered between sessions and is the same for every project. To use another
folder, click **Change LUT folder**. The dropdown only shows up when the folder contains at least
one `.cube` file.

## Set the LUT intensity

Once a LUT is loaded, the **Intensity** slider blends the graded image with the original, from
0 % (LUT off) to 100 % (full LUT, the default). Lower it when a conversion LUT feels too
contrasty or too saturated for your taste.

Moving the slider updates the preview immediately.

## Colour adjustments

The adjustments live in the collapsible **Correção Básica** section at the bottom of the panel.
It is closed by default, since the common case is a LUT and nothing else. Click it to open it.

{: .note }
The adjustment labels are currently shown in Portuguese in every interface language. The tables
below give the label exactly as it appears in the app, with its English meaning.

All sliders start at 0, which leaves the image untouched. **Redefinir ajustes** (reset
adjustments) puts every slider back to 0. The adjustments work with or without a LUT: exposure,
contrast and vignette are useful on their own.

### Luz (light)

All range from -100 to +100 %.

| Label in the app | Meaning | What it does |
|:--|:--|:--|
| **Exposição** | Exposure | Brightens or darkens like opening or closing the aperture, computed in linear light. The ends of the slider are 2 stops either way. |
| **Luminância** | Luminance | A plain brightness gain on the displayed image, up to 50 % either way. Same direction as exposure but a different curve, which is why both exist. |
| **Contraste** | Contrast | Spreads or squeezes the tones around middle grey, so the overall brightness stays put. At -100 the image goes flat grey. |
| **Destaques** | Highlights | Brightens or darkens the upper half of the tonal range. |
| **Sombras** | Shadows | Brightens or darkens the lower half of the tonal range. |
| **Brancos** | Whites | Like highlights, but only the brightest quarter of the tones. |
| **Pretos** | Blacks | Like shadows, but only the darkest quarter of the tones. |

The four tonal controls blend smoothly into each other, so there are no hard edges in skies and
gradients.

### Cor (colour)

All range from -100 to +100 %.

| Label in the app | Meaning | What it does |
|:--|:--|:--|
| **Temperatura** | Temperature | Positive warms the image (more red, less blue), negative cools it. |
| **Matiz** | Tint | Positive pushes towards magenta, negative towards green. Useful for correcting a green cast. |
| **Saturação** | Saturation | Makes every colour stronger or weaker. At -100 the image is black and white. |
| **Vivacidade** | Vibrance | Saturation that holds back on colours that are already strong, so skies and skin do not blow out first. |

### Efeito (effect)

Both range from 0 to 100 %.

| Label in the app | Meaning | What it does |
|:--|:--|:--|
| **Nitidez** | Sharpness | Sharpens the image. |
| **Vignette** | Vignette | Darkens the corners. Higher values darken more; the centre of the frame is never touched, and the vignette stays round on wide frames. |

## Order of operations

Colour operations give different results in a different order, so FPVFlow always applies them
in the same fixed sequence, after the frame has been stabilized:

1. Sharpness
2. Conversion LUT, blended by **Intensity**
3. Exposure, luminance, contrast
4. Blacks, shadows, highlights, whites
5. Temperature, tint
6. Saturation, vibrance
7. Vignette

Note that this is not exactly the order of the sliders in the panel: blacks and shadows act
before highlights and whites. Since the adjustments come after the LUT, they work on the
converted Rec.709 image, which is what you see in the preview. Sharpening is the exception: it
acts on the stabilized source, before the LUT.

Overlays drawn by the app on the preview, such as the safe area, are not graded.

## Bit depth and quality

Grading never lowers the quality of the footage:

- The grading math runs in floating point and is stored in a 16-bit RGB intermediate, so no
  precision is lost before the final conversion to the bit depth you export in.
- 10-bit footage stays 10-bit, and log footage converted to Rec.709 keeps smooth skies instead
  of showing banding.

To actually keep 10 bits, export with a codec and pixel format that support them, in
**Export settings**.

## Rec.709 tagging

When a LUT is loaded, the exported file is tagged as Rec.709. Players and editors then show the converted file correctly, instead of treating it as
log footage.

Without a LUT, the file keeps the colour tags of the source, even if you used the adjustments:
adjustments alone do not turn log into Rec.709, so tagging the file as Rec.709 would be wrong.

## Export speed and precise colour conversion

To apply a LUT or any adjustment, every frame has to be converted to RGB and back, on the CPU.
By default this conversion uses a fast filter and every CPU core. On 4K footage with a LUT, this
took exports from about 7 fps to about 30 fps on a Mac with an M4 Pro.

The **Precise color conversion (slower export)** checkbox, in the **Color** panel, makes the
conversion use the **Interpolation method** chosen in **Export settings** instead. The frame is
not resized at this step, only the colour channels are resampled, so the visible difference is
small and the cost in render speed is large.

{: .tip }
Leave **Precise color conversion (slower export)** off unless you see a problem in fine coloured
detail. Clips with no LUT and every adjustment at 0 skip the conversion entirely, so the option
makes no difference for them.

The checkbox is remembered between sessions and saved with each project and render queue job.

## Saved with the project and the queue

The LUT, its intensity, the adjustments you changed and the precise conversion option are saved
in the `.gyroflow` project file, and kept per clip in the render queue. Reopening a project or the
app restores each clip's grade.

The project stores the location of the `.cube` file, not a copy of it. If you move or rename the
LUT, load it again.
