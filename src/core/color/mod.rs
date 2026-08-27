// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Color processing: conversion LUTs and the adjustments applied after them.
//!
//! # Where this sits in the pipeline
//!
//! Everything here runs on the stabilized frame, in float, before the single
//! quantization to the output bit depth:
//!
//! ```text
//! warp/stabilization (float)
//!   -> conversion LUT + intensity mix          [now in Rec.709]
//!   -> per-pixel adjustments (light, color)
//!   -> spatial effects (sharpness, vignette)
//!   -> quantize to the output bit depth -> encode
//! ```
//!
//! Quantizing anywhere before the end reintroduces banding in skies and
//! gradients, which is exactly the material a log-to-Rec.709 conversion is used
//! on.

pub mod lut;
