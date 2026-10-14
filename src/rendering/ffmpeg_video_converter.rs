// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2021-2022 Adrian <adrian.eddy at gmail>

use ffmpeg_next::{ ffi, format, frame, software };
use crate::rendering::FFmpegError;

/// A swscale context that splits each frame across all cores.
///
/// `scaling::Context::run()` goes through the legacy `sws_scale()`, which is
/// single-threaded. The per-frame pixel format detour (the color grading one
/// converts every 4K frame to RGBA64 and back) was CPU bound on that one thread.
/// Initialized the legacy way, so the conversion itself is unchanged; only
/// `sws_scale_frame()` adds the slice threads.
pub struct ThreadedScaler(*mut ffi::SwsContext);
unsafe impl Send for ThreadedScaler { }
impl ThreadedScaler {
    pub fn new(src: (format::Pixel, u32, u32), dst: (format::Pixel, u32, u32), flags: software::scaling::flag::Flags) -> Result<Self, FFmpegError> {
        unsafe {
            let ctx = ffi::sws_alloc_context();
            if ctx.is_null() { return Err(FFmpegError::ConverterEmpty); }
            (*ctx).src_format = ffi::AVPixelFormat::from(src.0) as i32;
            (*ctx).src_w = src.1 as i32;
            (*ctx).src_h = src.2 as i32;
            (*ctx).dst_format = ffi::AVPixelFormat::from(dst.0) as i32;
            (*ctx).dst_w = dst.1 as i32;
            (*ctx).dst_h = dst.2 as i32;
            (*ctx).flags = flags.bits() as u32;
            (*ctx).threads = 0; // One per core
            let ret = ffi::sws_init_context(ctx, std::ptr::null_mut(), std::ptr::null_mut());
            if ret < 0 {
                ffi::sws_freeContext(ctx);
                return Err(FFmpegError::InternalError(ffmpeg_next::Error::from(ret)));
            }
            Ok(Self(ctx))
        }
    }
    pub fn run(&mut self, src: &frame::Video, dst: &mut frame::Video) -> Result<(), FFmpegError> {
        let ret = unsafe { ffi::sws_scale_frame(self.0, dst.as_mut_ptr(), src.as_ptr()) };
        if ret < 0 { return Err(FFmpegError::InternalError(ffmpeg_next::Error::from(ret))); }
        Ok(())
    }
}
impl Drop for ThreadedScaler {
    fn drop(&mut self) {
        unsafe { ffi::sws_freeContext(self.0); }
    }
}

#[derive(Default)]
pub struct Converter {
    pub convert_to: Option<ThreadedScaler>,
    pub convert_from: Option<ThreadedScaler>,
    pub sw_frame_converted: Option<frame::Video>,
    pub sw_frame_converted_out: Option<frame::Video>,
}
impl<'a> Converter {
    pub fn convert_pixel_format<F>(&mut self, frame: &mut frame::Video, out_frame: &mut frame::Video, format: format::Pixel, interpolation: software::scaling::flag::Flags, mut cb: F) -> Result<(), FFmpegError> where F: FnMut(&mut frame::Video, &mut frame::Video) + 'a {
        if frame.format() != format {
            if self.sw_frame_converted.is_none() {
                self.sw_frame_converted = Some(frame::Video::new(format, frame.width(), frame.height()));
                //self.convert_from = Some(software::converter((frame.width(), frame.height()), frame.format(), format)?);
                self.convert_from = Some(ThreadedScaler::new(
                    (frame.format(), frame.width(), frame.height()), // input
                    (format, frame.width(), frame.height()), // output
                    interpolation,
                )?);
            }

            if self.sw_frame_converted_out.is_none() {
                self.sw_frame_converted_out = Some(frame::Video::new(format, out_frame.width(), out_frame.height()));
                //self.convert_to = Some(software::converter((out_frame.width(), out_frame.height()), format, out_frame.format())?);
                self.convert_to = Some(ThreadedScaler::new(
                    (format, out_frame.width(), out_frame.height()), // input
                    (out_frame.format(), out_frame.width(), out_frame.height()), // output
                    interpolation,
                )?);
            }

            let sw_frame_converted = self.sw_frame_converted.as_mut().ok_or(FFmpegError::FrameEmpty)?;
            let sw_frame_converted_out = self.sw_frame_converted_out.as_mut().ok_or(FFmpegError::FrameEmpty)?;
            let convert_from = self.convert_from.as_mut().ok_or(FFmpegError::ConverterEmpty)?;
            let convert_to = self.convert_to.as_mut().ok_or(FFmpegError::ConverterEmpty)?;

            convert_from.run(frame, sw_frame_converted)?;

            cb(sw_frame_converted, sw_frame_converted_out);

            convert_to.run(sw_frame_converted_out, out_frame)?;
        } else {
            cb(frame, out_frame);
        }
        Ok(())
    }

    // Scale is only used for autosync
    pub fn scale(&mut self, frame: &mut frame::Video, format: format::Pixel, width: u32, height: u32) -> Result<frame::Video, FFmpegError> {
        if frame.width() != width || frame.height() != height || frame.format() != format {
            if self.sw_frame_converted.is_none() {
                self.sw_frame_converted = Some(frame::Video::new(format, width, height));
                self.convert_to = Some(ThreadedScaler::new(
                    (frame.format(), frame.width(), frame.height()), (format, width, height), software::scaling::Flags::BILINEAR,
                )?);
            }

            let sw_frame_converted = self.sw_frame_converted.as_mut().ok_or(FFmpegError::FrameEmpty)?;
            let convert_to = self.convert_to.as_mut().ok_or(FFmpegError::ConverterEmpty)?;

            convert_to.run(frame, sw_frame_converted)?;

            Ok(unsafe { frame::Video::wrap(ffi::av_frame_clone(sw_frame_converted.as_ptr())) })
        } else {
            Ok(unsafe { frame::Video::wrap(ffi::av_frame_clone(frame.as_ptr())) })
        }
    }
}
