// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright © 2026 Rodrigo Sclosa

//! Read the attitude track of a DJI O4 MP4 with the byte offset of every stored value, and
//! write a copy of the file with corrected values in place. Ported from o4fix `dji.py`.
//!
//! Deliberate differences from o4fix:
//! - The MP4 sample table is walked by telemetry-parser (the same samples Gyroflow reads), and
//!   the O3 / Avata 2 protobuf layouts are refused the way its DJI parser detects them.
//! - A quaternion with a component left out by proto3 (an exact 0.0) is kept with that
//!   component at 0.0, as prost decodes it; o4fix drops such entries. NaN and all-zero entries
//!   are skipped, as telemetry-parser skips them, and stay untouched in the file.
//! - Writing aligns the sign of each new value to the stored one, checks the bytes at every
//!   offset before overwriting them, and the result is read back and verified.

use std::fs::{ self, File, OpenOptions };
use std::io::{ self, BufReader, Read, Seek, SeekFrom, Write };
use std::path::{ Path, PathBuf };
use std::sync::{ Arc, atomic::{ AtomicBool, Ordering::Relaxed } };
use telemetry_parser::util;
use super::{ dsp, Burst, Quat4, Settings, UNIQUE_RATE_HZ };

/// Attitude quaternions as stored in the file, duplicates included, in file order.
#[derive(Clone, Debug, Default)]
pub struct AttitudeTrack {
    /// Stored values, normalized, `[w, x, y, z]`.
    pub quats: Vec<Quat4>,
    /// The stored `f32` values as they are in the file (0.0 for a left-out component).
    pub raw: Vec<[f32; 4]>,
    /// Absolute file offset of the 4 little-endian `f32` bytes of w, x, y, z. `None` when the
    /// component is 0.0 and proto3 left the field out.
    pub offsets: Vec<[Option<u64>; 4]>,
    /// Index of the metadata sample (video frame) each value belongs to.
    pub frames: Vec<u32>,
}

/// What `repair_file` did.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct FileRepairReport {
    pub stored_values: usize,
    pub unique_values: usize,
    pub bursts: Vec<Burst>,
    /// False when no burst was found: then no copy is written.
    pub written: bool,
}

// wm169 `ProductMeta` path to the quaternions: frame_meta(3) -> imu_frame_meta(3) ->
// imu_attitude_after_fusion(2) -> attitude(3) -> quaternion_w/x/y/z(1..4).
const FRAME_FIELD: u64 = 3;
const IMU_FIELD: u64 = 3;
const ATTITUDE_BLOCK_FIELD: u64 = 2;
const QUATERNION_FIELD: u64 = 3;

const NO_ATTITUDE: &str = "no 1 kHz attitude track: record with RockSteady and HorizonSteady off";

fn invalid_data(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// Value of one protobuf field.
enum Value<'a> {
    Varint,
    Fixed64,
    Bytes(&'a [u8]),
    Fixed32([u8; 4]),
}

fn read_varint(buf: &[u8], pos: &mut usize) -> io::Result<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *buf.get(*pos).ok_or_else(|| invalid_data("truncated protobuf varint"))?;
        *pos += 1;
        value |= ((byte & 0x7F) as u64) << shift;
        if byte < 0x80 {
            return Ok(value);
        }
    }
    Err(invalid_data("protobuf varint longer than 10 bytes"))
}

fn take<'a>(buf: &'a [u8], pos: &mut usize, len: u64) -> io::Result<&'a [u8]> {
    let end = usize::try_from(len).ok().and_then(|l| pos.checked_add(l)).filter(|&e| e <= buf.len())
        .ok_or_else(|| invalid_data("truncated protobuf field"))?;
    let out = &buf[*pos..end];
    *pos = end;
    Ok(out)
}

/// Call `f(field_number, value, offset_of_value)` for every field of one message level, in order.
/// `base` is the absolute file offset of `buf[0]`.
fn for_each_field<'a>(buf: &'a [u8], base: u64, mut f: impl FnMut(u64, Value<'a>, u64) -> io::Result<()>) -> io::Result<()> {
    let mut pos = 0;
    while pos < buf.len() {
        let key = read_varint(buf, &mut pos)?;
        let (number, wire_type) = (key >> 3, key & 7);
        if number == 0 {
            return Err(invalid_data("protobuf field number 0"));
        }
        let mut offset = base + pos as u64;
        let value = match wire_type {
            0 => { read_varint(buf, &mut pos)?; Value::Varint }
            1 => { take(buf, &mut pos, 8)?; Value::Fixed64 }
            2 => {
                let len = read_varint(buf, &mut pos)?;
                offset = base + pos as u64;
                Value::Bytes(take(buf, &mut pos, len)?)
            }
            5 => Value::Fixed32(take(buf, &mut pos, 4)?.try_into().unwrap()),
            _ => return Err(invalid_data(format!("unsupported protobuf wire type {wire_type}"))),
        };
        f(number, value, offset)?;
    }
    Ok(())
}

/// Call `f(submessage, offset)` for every length-delimited occurrence of `field` in `buf`.
fn for_each_submessage<'a>(buf: &'a [u8], base: u64, field: u64, mut f: impl FnMut(&'a [u8], u64) -> io::Result<()>) -> io::Result<()> {
    for_each_field(buf, base, |number, value, offset| match value {
        Value::Bytes(msg) if number == field => f(msg, offset),
        _ => Ok(()),
    })
}

fn parse_quaternion(msg: &[u8], base: u64, frame: u32, track: &mut AttitudeTrack) -> io::Result<()> {
    let mut raw = [0f32; 4];
    let mut offsets = [None; 4];
    for_each_field(msg, base, |number, value, offset| {
        if let (1..=4, Value::Fixed32(bytes)) = (number, value) {
            raw[number as usize - 1] = f32::from_le_bytes(bytes);
            offsets[number as usize - 1] = Some(offset);
        }
        Ok(())
    })?;
    if raw.iter().any(|v| v.is_nan()) || raw.iter().all(|&v| v == 0.0) {
        return Ok(());
    }
    let q = raw.map(|v| v as f64);
    let norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    if !norm.is_finite() || norm < 0.5 {
        return Err(invalid_data(format!("invalid attitude quaternion {raw:?}")));
    }
    track.quats.push(q.map(|v| v / norm));
    track.raw.push(raw);
    track.offsets.push(offsets);
    track.frames.push(frame);
    Ok(())
}

/// Append the attitude quaternions of one wm169 metadata sample. `base_offset` is the file
/// offset of `data[0]`. On error the track is left as it was.
pub(crate) fn parse_sample(data: &[u8], base_offset: u64, frame: u32, track: &mut AttitudeTrack) -> io::Result<()> {
    let len = track.quats.len();
    let result = for_each_submessage(data, base_offset, FRAME_FIELD, |frame_msg, frame_offset| {
        for_each_submessage(frame_msg, frame_offset, IMU_FIELD, |imu_msg, imu_offset| {
            for_each_submessage(imu_msg, imu_offset, ATTITUDE_BLOCK_FIELD, |block_msg, block_offset| {
                for_each_submessage(block_msg, block_offset, QUATERNION_FIELD, |quat_msg, quat_offset| {
                    parse_quaternion(quat_msg, quat_offset, frame, track)
                })
            })
        })
    });
    if result.is_err() {
        track.quats.truncate(len);
        track.raw.truncate(len);
        track.offsets.truncate(len);
        track.frames.truncate(len);
    }
    result
}

/// `<dir>/<stem>_gyrofix.<ext>` next to `src`.
pub fn corrected_copy_path(src: &Path) -> PathBuf {
    let mut name = src.file_stem().unwrap_or_default().to_os_string();
    name.push(super::COPY_SUFFIX);
    if let Some(ext) = src.extension() {
        name.push(".");
        name.push(ext);
    }
    src.with_file_name(name)
}

/// Read the 1 kHz attitude track of a DJI O4 / O4 Pro MP4 (wm169 layout).
pub fn read_track(path: &Path, cancel_flag: Arc<AtomicBool>) -> io::Result<AttitudeTrack> {
    read_track_with_progress(path, |_| (), cancel_flag)
}

fn read_track_with_progress<F: Fn(f64)>(path: &Path, progress: F, cancel_flag: Arc<AtomicBool>) -> io::Result<AttitudeTrack> {
    let file = File::open(path)?;
    let size = file.metadata()?.len() as usize;
    let mut stream = BufReader::with_capacity(512 * 1024, file);
    let mut track = AttitudeTrack::default();
    let mut samples = 0usize;
    let mut result = Ok(());
    // Our own flag, so an error can stop the walk without touching the caller's.
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    util::get_metadata_track_samples(&mut stream, size, true, |info, data: &[u8], file_position: u64, _| {
        if cancel_flag.load(Relaxed) || result.is_err() {
            stop2.store(true, Relaxed);
            return;
        }
        if size > 0 {
            progress(file_position as f64 / size as f64);
        }
        // Same detection as telemetry-parser's DJI parser: the other layouts name themselves
        // in the first sample.
        if samples == 0 && data.len() > 64 && [&b"oq101"[..], b"WA530", b"wa530"].iter().any(|tag| data[..64].windows(tag.len()).any(|w| w == *tag)) {
            result = Err(invalid_data("unsupported DJI metadata layout: only DJI O4 / O4 Pro (wm169) clips can be repaired"));
            stop2.store(true, Relaxed);
            return;
        }
        samples += 1;
        result = parse_sample(data, file_position, info.sample_index as u32, &mut track).map_err(|e| {
            invalid_data(format!("corrupt DJI attitude metadata in frame {} (file offset {file_position}): {e}", info.sample_index))
        });
        if result.is_err() {
            stop2.store(true, Relaxed);
        }
    }, stop)?;
    if cancel_flag.load(Relaxed) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
    }
    result?;
    if samples == 0 {
        return Err(invalid_data("no metadata track in this file (is it an original DJI O4 recording?)"));
    }
    if track.quats.is_empty() {
        return Err(invalid_data(NO_ATTITUDE));
    }
    Ok(track)
}

/// `new` with the sign of `stored`, so the file keeps its sign convention (q and -q are the
/// same rotation).
fn sign_aligned(stored: &Quat4, new: &Quat4) -> Quat4 {
    let dot: f64 = stored.iter().zip(new).map(|(a, b)| a * b).sum();
    if dot < 0.0 { new.map(|v| -v) } else { *new }
}

/// `path` resolved, also when the file does not exist yet.
fn canonical(path: &Path) -> io::Result<PathBuf> {
    match fs::canonicalize(path) {
        Ok(p) => Ok(p),
        Err(_) => {
            let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            Ok(fs::canonicalize(parent)?.join(path.file_name().unwrap_or_default()))
        }
    }
}

/// Copy `src` to `dst` with every stored quaternion replaced by `quats` (same length as
/// `track.quats`), keeping the file layout and size. Built in a temporary file next to `dst`
/// and renamed over it only when complete, so a failure keeps any previous `dst`.
pub fn write_track(src: &Path, dst: &Path, track: &AttitudeTrack, quats: &[Quat4]) -> io::Result<()> {
    let tmp = write_temp_copy(src, dst, track, quats)?;
    fs::rename(&tmp, dst).inspect_err(|_| { let _ = fs::remove_file(&tmp); })
}

/// The work of `write_track` up to the rename: returns the complete temporary file.
fn write_temp_copy(src: &Path, dst: &Path, track: &AttitudeTrack, quats: &[Quat4]) -> io::Result<PathBuf> {
    let invalid_input = |msg: &str| io::Error::new(io::ErrorKind::InvalidInput, msg.to_string());
    if fs::canonicalize(src)? == canonical(dst)? {
        return Err(invalid_input("refusing to overwrite the input file; choose a different output path"));
    }
    let n = track.quats.len();
    if quats.len() != n || track.raw.len() != n || track.offsets.len() != n {
        return Err(invalid_input("corrected values do not match the attitude track"));
    }
    // (offset, bytes expected there, new bytes), sorted by offset.
    let mut patches = Vec::with_capacity(n * 4);
    for i in 0..n {
        let q = sign_aligned(&track.quats[i], &quats[i]);
        for c in 0..4 {
            if !q[c].is_finite() {
                return Err(invalid_input("corrected attitude contains non-finite values"));
            }
            match track.offsets[i][c] {
                Some(offset) => patches.push((offset, track.raw[i][c].to_le_bytes(), (q[c] as f32).to_le_bytes())),
                None if q[c].abs() > 1e-7 => return Err(invalid_input("cannot write a quaternion component the file left out")),
                None => { }
            }
        }
    }
    patches.sort_unstable_by_key(|p| p.0);
    if patches.windows(2).any(|w| w[1].0 < w[0].0 + 4) {
        return Err(invalid_input("overlapping attitude offsets"));
    }

    let name = dst.file_name().ok_or_else(|| invalid_input("output path has no file name"))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(".tmp");
    let tmp = dst.with_file_name(tmp_name);
    // Removing a stale temporary file first lets `fs::copy` clone the file on APFS. `dst` stays
    // until the rename replaces it.
    match fs::remove_file(&tmp) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => { }
    }

    let write = || -> io::Result<()> {
        fs::copy(src, &tmp)?;
        // `fs::copy` copies the source permissions: a read-only clip would give a read-only copy.
        let mut perms = fs::metadata(&tmp)?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(perms.mode() | 0o200);
        }
        #[cfg(not(unix))]
        perms.set_readonly(false);
        fs::set_permissions(&tmp, perms)?;
        let mut file = OpenOptions::new().read(true).write(true).open(&tmp)?;
        // Patch runs of nearby values (one metadata sample) with one read and one write.
        const MAX_GAP: u64 = 4096;
        const MAX_SPAN: u64 = 1 << 20;
        let mut buf = Vec::new();
        let mut i = 0;
        while i < patches.len() {
            let start = patches[i].0;
            let mut j = i + 1;
            while j < patches.len() && patches[j].0 <= patches[j - 1].0 + 4 + MAX_GAP && patches[j].0 + 4 - start <= MAX_SPAN {
                j += 1;
            }
            buf.resize((patches[j - 1].0 + 4 - start) as usize, 0);
            file.seek(SeekFrom::Start(start))?;
            file.read_exact(&mut buf)?;
            for (offset, expected, new) in &patches[i..j] {
                let k = (offset - start) as usize;
                if buf[k..k + 4] != expected[..] {
                    return Err(invalid_data(format!("the file does not hold the expected attitude value at offset {offset}")));
                }
                buf[k..k + 4].copy_from_slice(new);
            }
            file.seek(SeekFrom::Start(start))?;
            file.write_all(&buf)?;
            i = j;
        }
        file.flush()?;
        file.sync_all()?;
        drop(file);
        if fs::metadata(&tmp)?.len() != fs::metadata(src)?.len() {
            return Err(io::Error::new(io::ErrorKind::Other, "the corrected copy changed size"));
        }
        Ok(())
    };
    match write() {
        Ok(()) => Ok(tmp),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

/// Read, correct and, when bursts were found, write the corrected copy at `dst`.
pub fn repair_file<F: Fn(f64)>(src: &Path, dst: &Path, settings: &Settings, progress: F, cancel_flag: Arc<AtomicBool>) -> io::Result<FileRepairReport> {
    let cancelled = || if cancel_flag.load(Relaxed) { Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled")) } else { Ok(()) };
    let track = read_track_with_progress(src, |p| progress(p * 0.45), cancel_flag.clone())?;
    let (unique, unique_of_entry) = dsp::dedup(&track.quats);
    let mut report = FileRepairReport { stored_values: track.quats.len(), unique_values: unique.len(), ..Default::default() };
    progress(0.45);
    cancelled()?;
    let Some(correction) = dsp::repair(&unique, UNIQUE_RATE_HZ, settings) else {
        progress(1.0);
        return Ok(report);
    };
    if correction.quats.len() != unique.len() || unique_of_entry.len() != track.quats.len() || unique_of_entry.iter().any(|&u| u >= unique.len()) {
        return Err(io::Error::new(io::ErrorKind::Other, "attitude correction returned a wrong number of values"));
    }
    report.bursts = correction.bursts;
    let quats: Vec<Quat4> = unique_of_entry.iter().map(|&u| correction.quats[u]).collect();
    progress(0.5);
    cancelled()?;
    let tmp = write_temp_copy(src, dst, &track, &quats)?;
    progress(0.6);

    // Read the copy back before it replaces `dst`: same entries at the same offsets, holding
    // what we meant to write.
    let verify = || -> io::Result<()> {
        let written = read_track_with_progress(&tmp, |p| progress(0.6 + p * 0.4), cancel_flag.clone())?;
        if written.offsets != track.offsets {
            return Err(io::Error::new(io::ErrorKind::Other, "the corrected copy has a different attitude layout"));
        }
        for i in 0..quats.len() {
            let q = sign_aligned(&track.quats[i], &quats[i]);
            if (0..4).any(|c| (written.raw[i][c] as f64 - (q[c] as f32) as f64).abs() > 1e-6) {
                return Err(io::Error::new(io::ErrorKind::Other, format!("verification of the corrected copy failed at value {i}")));
            }
        }
        Ok(())
    };
    if let Err(e) = verify().and_then(|_| fs::rename(&tmp, dst)) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    report.written = true;
    progress(1.0);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn varint(mut v: u64, out: &mut Vec<u8>) {
        while v >= 0x80 {
            out.push((v as u8) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }
    fn key(field: u64, wire: u64, out: &mut Vec<u8>) { varint(field << 3 | wire, out) }
    fn msg(field: u64, payload: &[u8], out: &mut Vec<u8>) {
        key(field, 2, out);
        varint(payload.len() as u64, out);
        out.extend_from_slice(payload);
    }
    fn fixed32(field: u64, v: f32, out: &mut Vec<u8>) {
        key(field, 5, out);
        out.extend_from_slice(&v.to_le_bytes());
    }
    /// One unknown field of every wire type, with numbers not used at any level of the path.
    fn unknown_fields(out: &mut Vec<u8>) {
        key(9, 0, out); varint(300, out);
        key(10, 1, out); out.extend_from_slice(&[0x11; 8]);
        msg(11, &[0x42; 200], out); // two-byte length
        key(12, 5, out); out.extend_from_slice(&[0x22; 4]);
    }
    /// Quaternion message; `None` components are left out.
    fn quat(q: [Option<f32>; 4]) -> Vec<u8> {
        let mut out = Vec::new();
        unknown_fields(&mut out);
        for (c, v) in q.iter().enumerate() {
            if let Some(v) = v { fixed32(c as u64 + 1, *v, &mut out); }
            if c == 1 { unknown_fields(&mut out); }
        }
        out
    }
    fn nest(field: u64, children: &[Vec<u8>], out: &mut Vec<u8>) {
        let mut payload = Vec::new();
        unknown_fields(&mut payload);
        for child in children {
            msg(field, child, &mut payload);
            unknown_fields(&mut payload);
        }
        out.extend_from_slice(&payload);
    }
    /// A ProductMeta sample with the given quaternion messages split over the repeated levels.
    fn sample(quats: &[Vec<u8>]) -> Vec<u8> {
        let mut blocks = Vec::new();
        for chunk in quats.chunks(3) {
            let mut block = Vec::new();
            nest(QUATERNION_FIELD, chunk, &mut block);
            blocks.push(block);
        }
        let mut imus = Vec::new();
        for chunk in blocks.chunks(2) {
            let mut imu = Vec::new();
            nest(ATTITUDE_BLOCK_FIELD, chunk, &mut imu);
            // Same field number as the attitude block, other wire type: ignored.
            key(ATTITUDE_BLOCK_FIELD, 0, &mut imu); varint(1, &mut imu);
            imus.push(imu);
        }
        let mut frames = Vec::new();
        for chunk in imus.chunks(1) {
            let mut frame = Vec::new();
            nest(IMU_FIELD, chunk, &mut frame);
            frames.push(frame);
        }
        let mut out = Vec::new();
        key(FRAME_FIELD, 5, &mut out); out.extend_from_slice(&[0; 4]);
        nest(FRAME_FIELD, &frames, &mut out);
        out
    }

    #[test]
    fn parse_sample_walks_nested_repeated_messages() {
        let values: Vec<[Option<f32>; 4]> = vec![
            [Some(0.9), Some(0.1), Some(-0.3), Some(0.2)],
            [Some(0.5), Some(0.5), None, Some(-0.5)],            // y left out by proto3
            [Some(f32::NAN), Some(0.1), Some(0.2), Some(0.3)],  // skipped
            [Some(0.0), Some(0.0), Some(0.0), Some(0.0)],        // skipped
            [None, None, None, None],                            // skipped (all zero)
            [None, Some(1.0), Some(0.0), Some(0.0)],
            [Some(-0.6), Some(0.0), Some(0.8), Some(0.0)],
        ];
        let quats: Vec<Vec<u8>> = values.iter().map(|q| quat(*q)).collect();
        let data = sample(&quats);
        let base = 1_000_003;
        let mut track = AttitudeTrack::default();
        parse_sample(&data, base, 7, &mut track).unwrap();

        let kept = [0, 1, 5, 6];
        assert_eq!(track.quats.len(), kept.len());
        assert_eq!(track.raw.len(), kept.len());
        assert_eq!(track.frames, vec![7; kept.len()]);
        for (k, &i) in kept.iter().enumerate() {
            let expected = values[i].map(|v| v.unwrap_or(0.0));
            assert_eq!(track.raw[k], expected);
            let norm = expected.iter().map(|&v| (v as f64).powi(2)).sum::<f64>().sqrt();
            for c in 0..4 {
                assert!((track.quats[k][c] - expected[c] as f64 / norm).abs() < 1e-12);
                match track.offsets[k][c] {
                    Some(offset) => {
                        let at = (offset - base) as usize;
                        assert_eq!(data[at..at + 4], expected[c].to_le_bytes());
                        assert_eq!(data[at - 1], (c as u8 + 1) << 3 | 5, "offset points right after the field key");
                    }
                    None => assert!(values[i][c].is_none()),
                }
            }
        }
        // File order.
        assert!(track.offsets.windows(2).all(|w| w[0][1].unwrap() < w[1][1].unwrap()));

        // A second sample appends.
        parse_sample(&data, base + 10_000, 8, &mut track).unwrap();
        assert_eq!(track.quats.len(), 2 * kept.len());
        assert_eq!(track.frames[kept.len()], 8);
        assert_eq!(track.offsets[kept.len()][0], track.offsets[0][0].map(|o| o + 10_000));
    }

    #[test]
    fn parse_sample_rejects_malformed_input() {
        let data = sample(&vec![quat([Some(0.9), Some(0.1), Some(-0.3), Some(0.2)]); 4]);
        let mut track = AttitudeTrack::default();
        parse_sample(&data, 0, 0, &mut track).unwrap();
        let reference = track.quats.clone();

        // Any truncation: an error or a shorter result, never a panic, and never a partial track.
        for len in 0..data.len() {
            let mut t = track.clone();
            match parse_sample(&data[..len], 0, 1, &mut t) {
                Ok(()) => assert!(t.quats.len() <= 2 * reference.len()),
                Err(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::InvalidData);
                    assert_eq!(t.quats, reference);
                    assert_eq!(t.offsets.len(), reference.len());
                }
            }
        }
        // Cut inside the last float: malformed.
        let last = track.offsets.last().unwrap()[3].unwrap() as usize;
        assert!(parse_sample(&data[..last + 2], 0, 1, &mut track.clone()).is_err());

        let bad: Vec<Vec<u8>> = vec![
            vec![0x1A, 0x05, 0x01],                   // length beyond the buffer
            vec![0x1A, 0xFF],                         // truncated length varint
            vec![0x0D, 0x00, 0x00],                   // truncated fixed32
            vec![0x09, 0x00],                         // truncated fixed64
            vec![0x1B],                               // group start (wire type 3)
            vec![0x1C],                               // group end (wire type 4)
            vec![0x1F],                               // wire type 7
            vec![0x00, 0x00],                         // field number 0
            vec![0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01], // varint > 10 bytes
            vec![0x1A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01], // huge length
        ];
        for b in &bad {
            let mut t = AttitudeTrack::default();
            assert!(parse_sample(b, 0, 0, &mut t).is_err(), "{b:x?}");
            assert!(t.quats.is_empty());
        }
        // The same errors nested at the quaternion level.
        let mut inner = quat([Some(0.9), Some(0.1), Some(-0.3), Some(0.2)]);
        inner.push(0x1B);
        let mut t = track.clone();
        assert!(parse_sample(&sample(&[inner]), 0, 0, &mut t).is_err());
        assert_eq!(t.quats, reference);
        // Infinite or tiny quaternions are refused.
        for q in [[Some(f32::INFINITY), Some(0.0), Some(0.0), Some(0.0)], [Some(0.1), Some(0.0), Some(0.0), Some(0.0)]] {
            assert!(parse_sample(&sample(&[quat(q)]), 0, 0, &mut AttitudeTrack::default()).is_err());
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fpvflow-gyrogate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A file of filler bytes holding the given stored values at the given offsets.
    fn synthetic_file(path: &Path, size: usize, entries: &[([f32; 4], [Option<u64>; 4])]) -> AttitudeTrack {
        let mut bytes: Vec<u8> = (0..size).map(|i| (i * 31 % 251) as u8).collect();
        let mut track = AttitudeTrack::default();
        for (frame, (raw, offsets)) in entries.iter().enumerate() {
            for c in 0..4 {
                if let Some(o) = offsets[c] {
                    bytes[o as usize..o as usize + 4].copy_from_slice(&raw[c].to_le_bytes());
                }
            }
            let q = raw.map(|v| v as f64);
            let n = q.iter().map(|v| v * v).sum::<f64>().sqrt();
            track.quats.push(q.map(|v| v / n));
            track.raw.push(*raw);
            track.offsets.push(*offsets);
            track.frames.push(frame as u32);
        }
        fs::write(path, bytes).unwrap();
        track
    }

    #[test]
    fn write_track_patches_values_in_place() {
        let dir = temp_dir("write");
        let src = dir.join("clip.MP4");
        let dst = dir.join("clip_gyrofix.MP4");
        let entries = [
            ([0.6, 0.0, 0.8, 0.0], [Some(100), Some(105), Some(110), Some(115)]),
            ([0.5, 0.5, 0.0, -0.70710677], [Some(3000), Some(3005), None, Some(3010)]),
            // Stored out of offset order.
            ([1.0, 0.0, 0.0, 0.0], [Some(20000), Some(20005), Some(20010), Some(20015)]),
            ([0.0, 1.0, 0.0, 0.0], [Some(200), Some(205), Some(210), Some(215)]),
        ];
        let track = synthetic_file(&src, 30000, &entries);
        let s = 0.5f64.sqrt();
        let quats = vec![
            [0.8, 0.0, 0.6, 0.0],
            [-0.5, -0.5, 0.0, 0.70710678], // opposite sign: written as [0.5, 0.5, 0, -0.707]
            [s, s, 0.0, 0.0],
            [0.0, -s, 0.0, -s],             // dot < 0 with [0, 1, 0, 0]: negated
        ];
        let expected: [[f32; 4]; 4] = [
            [0.8, 0.0, 0.6, 0.0],
            [0.5, 0.5, 0.0, -0.70710678],
            [s as f32, s as f32, 0.0, 0.0],
            [-0.0, s as f32, -0.0, s as f32], // negated zeros keep their sign bit
        ];
        fs::write(dir.join(".clip_gyrofix.MP4.tmp"), b"stale").unwrap();
        fs::write(&dst, b"old output").unwrap();

        write_track(&src, &dst, &track, &quats).unwrap();
        let before = fs::read(&src).unwrap();
        let after = fs::read(&dst).unwrap();
        assert_eq!(before.len(), after.len());
        assert!(!dir.join(".clip_gyrofix.MP4.tmp").exists());
        let mut patched = before.clone();
        for (i, (_, offsets)) in entries.iter().enumerate() {
            for c in 0..4 {
                if let Some(o) = offsets[c] {
                    patched[o as usize..o as usize + 4].copy_from_slice(&expected[i][c].to_le_bytes());
                }
            }
        }
        assert!(patched == after, "only the attitude values change");
        assert!(patched != before);

        // Same file, also through another spelling of the path.
        let same = dir.join(".").join("clip.MP4");
        assert_eq!(write_track(&src, &same, &track, &quats).unwrap_err().kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read(&src).unwrap(), before);

        // Wrong length.
        assert!(write_track(&src, &dst, &track, &quats[..3]).is_err());

        // The bytes at an offset are not the stored value: refused, the previous output kept,
        // nothing left behind.
        let mut wrong = track.clone();
        wrong.offsets[2][1] = Some(20006);
        let err = write_track(&src, &dst, &wrong, &quats).unwrap_err();
        assert!(err.to_string().contains("offset 20006"), "{err}");
        assert!(fs::read(&dst).unwrap() == after, "a failed write keeps the previous output");
        assert!(!dir.join(".clip_gyrofix.MP4.tmp").exists());
        fs::remove_file(&dst).unwrap();

        // Overlapping offsets.
        let mut overlap = track.clone();
        overlap.offsets[0][1] = Some(102);
        assert!(write_track(&src, &dst, &overlap, &quats).is_err());

        // A left-out component cannot receive a non-zero value.
        let mut q = quats.clone();
        q[1] = [0.5, 0.5, 0.1, -0.7];
        assert!(write_track(&src, &dst, &track, &q).is_err());
        assert!(!dst.exists());

        // Offset past the end of the file.
        let mut past = track.clone();
        past.offsets[2][3] = Some(29998);
        assert!(write_track(&src, &dst, &past, &quats).is_err());
        assert!(!dst.exists() && !dir.join(".clip_gyrofix.MP4.tmp").exists());

        // A read-only source can still be repaired; the copy is writable by its owner.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&src, fs::Permissions::from_mode(0o444)).unwrap();
            write_track(&src, &dst, &track, &quats).unwrap();
            assert!(fs::read(&dst).unwrap() == after);
            assert_eq!(fs::metadata(&dst).unwrap().permissions().mode() & 0o777, 0o644);
        }

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn corrected_copy_path_cases() {
        let p = |s: &str| corrected_copy_path(Path::new(s));
        assert_eq!(p("/a/b/DJI_0001.MP4"), Path::new("/a/b/DJI_0001_gyrofix.MP4"));
        assert_eq!(p("/a/b/clip.mp4"), Path::new("/a/b/clip_gyrofix.mp4"));
        assert_eq!(p("clip.Mp4"), Path::new("clip_gyrofix.Mp4"));
        assert_eq!(p("/a/my.clip.v2.MOV"), Path::new("/a/my.clip.v2_gyrofix.MOV"));
        assert_eq!(p("/a/clip"), Path::new("/a/clip_gyrofix"));
        assert_eq!(p("/a/.hidden"), Path::new("/a/.hidden_gyrofix"));
    }

    /// ~200k entries spread like a 3-minute clip: the write must stay well under a second.
    #[test]
    #[ignore]
    fn write_track_speed() {
        let dir = temp_dir("speed");
        let src = dir.join("big.MP4");
        let (samples, per_sample, gap) = (11_000u64, 18u64, 8_000u64);
        let entries: Vec<([f32; 4], [Option<u64>; 4])> = (0..samples * per_sample).map(|i| {
            let o = (i / per_sample) * gap + (i % per_sample) * 40;
            ([0.6, 0.0, 0.8, 0.0], [Some(o), Some(o + 5), Some(o + 10), Some(o + 15)])
        }).collect();
        let track = synthetic_file(&src, (samples * gap) as usize, &entries);
        let quats = vec![[0.8, 0.0, 0.6, 0.0]; entries.len()];
        let t = std::time::Instant::now();
        write_track(&src, &dir.join("out.MP4"), &track, &quats).unwrap();
        let elapsed = t.elapsed();
        println!("write_track: {} entries in {:.3} s", entries.len(), elapsed.as_secs_f64());
        assert!(elapsed.as_secs_f64() < 1.0);
        fs::remove_dir_all(&dir).unwrap();
    }

    fn telemetry_parser_quat_count(path: &Path) -> usize {
        use telemetry_parser::tags_impl::{ GetWithType, GroupId, TagId, TimeQuaternion };
        let mut file = File::open(path).unwrap();
        let size = file.metadata().unwrap().len() as usize;
        let input = telemetry_parser::Input::from_stream(&mut file, size, path, |_| (), Arc::new(AtomicBool::new(false))).unwrap();
        let mut count = 0;
        for info in input.samples.as_ref().unwrap() {
            if let Some(map) = info.tag_map.as_ref().and_then(|m| m.get(&GroupId::Quaternion)) {
                if let Some(arr) = map.get_t(TagId::Data) as Option<&Vec<TimeQuaternion<f64>>> {
                    count += arr.len();
                }
            }
        }
        count
    }

    /// Real clips: `FPVFLOW_GYROGATE_DIR` holds o4lite_53-58s.MP4, o4pro_13-17s.MP4 and their
    /// o4fix outputs (*.o4fix.MP4).
    #[test]
    #[ignore]
    fn real_clips() {
        let Ok(samples_dir) = std::env::var("FPVFLOW_GYROGATE_DIR") else {
            println!("FPVFLOW_GYROGATE_DIR not set, skipping");
            return;
        };
        let samples_dir = PathBuf::from(samples_dir);
        let no_cancel = || Arc::new(AtomicBool::new(false));
        let dir = temp_dir("real");

        // O4 Lite: 3 bursts.
        let src = samples_dir.join("o4lite_53-58s.MP4");
        let t = std::time::Instant::now();
        let track = read_track(&src, no_cancel()).unwrap();
        println!("o4lite read_track: {:.3} s", t.elapsed().as_secs_f64());
        assert_eq!(track.quats.len(), 10799);
        assert_eq!(dsp::dedup(&track.quats).0.len(), 5400);

        let dst = dir.join(corrected_copy_path(&src).file_name().unwrap());
        let t = std::time::Instant::now();
        let report = repair_file(&src, &dst, &Settings::default(), |_| (), no_cancel()).unwrap();
        println!("o4lite repair_file: {:.3} s, {:?}", t.elapsed().as_secs_f64(), report);
        assert!(report.written);
        assert_eq!((report.stored_values, report.unique_values, report.bursts.len()), (10799, 5400, 3));

        let ours = read_track(&dst, no_cancel()).unwrap();
        let o4fix = read_track(&samples_dir.join("o4lite_53-58s.o4fix.MP4"), no_cancel()).unwrap();
        assert_eq!(ours.offsets, track.offsets);
        assert_eq!(o4fix.offsets, track.offsets);
        let mut max_diff = 0f64;
        for (a, b) in ours.quats.iter().zip(&o4fix.quats) {
            let b = sign_aligned(a, b);
            max_diff = (0..4).fold(max_diff, |m, c| m.max((a[c] - b[c]).abs()));
        }
        println!("o4lite max |ours - o4fix| component difference: {max_diff:e}");
        assert!(max_diff < 1e-5);

        let src_bytes = fs::read(&src).unwrap();
        let dst_bytes = fs::read(&dst).unwrap();
        assert_eq!(src_bytes.len(), dst_bytes.len());
        assert_eq!(fs::metadata(samples_dir.join("o4lite_53-58s.o4fix.MP4")).unwrap().len() as usize, src_bytes.len());
        let mut expected = src_bytes.clone();
        for offsets in &track.offsets {
            for o in offsets.iter().flatten() {
                let o = *o as usize;
                expected[o..o + 4].copy_from_slice(&dst_bytes[o..o + 4]);
            }
        }
        assert!(expected == dst_bytes, "bytes outside the attitude values are identical");
        drop((src_bytes, dst_bytes, expected));

        let t = std::time::Instant::now();
        let (n_src, n_dst) = (telemetry_parser_quat_count(&src), telemetry_parser_quat_count(&dst));
        println!("telemetry-parser quaternions: source {n_src}, corrected {n_dst} ({:.3} s)", t.elapsed().as_secs_f64());
        assert!(n_src > 0);
        assert_eq!(n_src, n_dst);

        // Cancelled while the copy is read back: the previous output stays, no temp file.
        fs::write(&dst, b"previous output").unwrap();
        let cancel = no_cancel();
        let cancel2 = cancel.clone();
        let err = repair_file(&src, &dst, &Settings::default(), move |p| if p > 0.7 { cancel2.store(true, Relaxed) }, cancel).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert_eq!(fs::read(&dst).unwrap(), b"previous output");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no temp file left behind");

        // A malformed metadata sample: the error says where.
        let corrupt = dir.join("corrupt.MP4");
        fs::copy(&src, &corrupt).unwrap();
        let mut bytes = fs::read(&corrupt).unwrap();
        let at = track.offsets[100][0].unwrap() as usize - 1;
        assert_eq!(bytes[at], 0x0D);
        bytes[at] = 0x0B; // field 1, wire type 3
        fs::write(&corrupt, bytes).unwrap();
        let err = read_track(&corrupt, no_cancel()).unwrap_err().to_string();
        println!("corrupt sample: {err}");
        assert!(err.contains(&format!("frame {}", track.frames[100])) && err.contains("file offset") && err.contains("wire type 3"), "{err}");
        fs::remove_file(&corrupt).unwrap();

        // O4 Pro: no burst, nothing written.
        let src = samples_dir.join("o4pro_13-17s.MP4");
        let track = read_track(&src, no_cancel()).unwrap();
        assert_eq!(track.quats.len(), 9600);
        assert_eq!(dsp::dedup(&track.quats).0.len(), 4801);
        let dst = dir.join("o4pro_gyrofix.MP4");
        let report = repair_file(&src, &dst, &Settings::default(), |_| (), no_cancel()).unwrap();
        println!("o4pro: {report:?}");
        assert!(!report.written && report.bursts.is_empty());
        assert!(!dst.exists());

        // Optional long clean clip (read only).
        if let Ok(clean) = std::env::var("FPVFLOW_GYROGATE_CLEAN_CLIP") {
            let t = std::time::Instant::now();
            let track = read_track(Path::new(&clean), no_cancel()).unwrap();
            let (unique, _) = dsp::dedup(&track.quats);
            println!("clean clip: {} stored, {} unique, read in {:.3} s", track.quats.len(), unique.len(), t.elapsed().as_secs_f64());
            let t = std::time::Instant::now();
            let bursts = dsp::repair(&unique, UNIQUE_RATE_HZ, &Settings::default()).map(|c| c.bursts.len()).unwrap_or(0);
            println!("clean clip: {bursts} bursts, repair in {:.3} s", t.elapsed().as_secs_f64());
        }

        fs::remove_dir_all(&dir).unwrap();
    }
}
