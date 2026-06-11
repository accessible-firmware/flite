//! Serialize a flite `cst_wave` into a canonical 16-bit PCM WAV byte buffer.

use crate::CstWave;
use alloc::vec::Vec;

/// Build a little-endian 16-bit PCM WAV file in memory from `w`.
///
/// # Safety
/// `w.samples` must point to `w.num_samples * w.num_channels` valid `i16`s.
pub unsafe fn wave_to_wav(w: &CstWave) -> Vec<u8> {
    let channels = w.num_channels.max(1) as u16;
    let rate = w.sample_rate.max(1) as u32;
    let n = w.num_samples.max(0) as usize * channels as usize;
    let bits_per_sample: u16 = 16;
    let block_align: u16 = channels * (bits_per_sample / 8);
    let byte_rate: u32 = rate * block_align as u32;
    let data_bytes: u32 = (n * 2) as u32;

    let mut out = Vec::with_capacity(44 + data_bytes as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes()); // chunk size
    out.extend_from_slice(b"WAVE");
    // fmt subchunk
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data subchunk
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    if !w.samples.is_null() {
        let samples = core::slice::from_raw_parts(w.samples, n);
        for s in samples {
            out.extend_from_slice(&s.to_le_bytes());
        }
    }
    out
}
