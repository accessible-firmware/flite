#![cfg_attr(not(feature = "std"), no_std)]
#![feature(c_variadic)]
//! Text-to-speech for freestanding x86_64 targets, built on the CMU flite engine.
//!
//! This crate links the cross-compiled flite C archive (`libflite_coff.a` for
//! `x86_64-unknown-uefi`, `libflite_elf.a` for `x86_64-unknown-none`) and
//! supplies the C runtime it needs (allocation, `str`/`mem`, math, etc.), then
//! exposes a small safe API. Synthesis returns 16-bit signed PCM samples that
//! the host application can hand to its own audio driver.
//!
//! ```ignore
//! let voice = flite_freestanding::init().expect("voice");
//! let wave = voice.synthesize("hello world").expect("synth");
//! my_audio_driver.play(wave.samples(), wave.sample_rate(), wave.num_channels());
//! ```
//!
//! Build for a freestanding target with a nightly toolchain. The matching flite C
//! archive must exist next to this crate (`../libflite_coff.a` / `../libflite_elf.a`);
//! regenerate them with `make -C freestanding-port coff` / `make -C freestanding-port elf`. See
//! `freestanding-port/README.md`.
//!
//! ## std vs. no_std
//!
//! By default the crate is no_std + `extern crate alloc`. The `std` feature
//! re-enables the historical UEFI-std build (`std::ffi::CString`,
//! `std::io::stdout()` for the printf shim) — that's what the `synth` example
//! and `consumer-demo` crate use. Embedders linking this crate into a larger
//! freestanding binary (e.g. a GRUB module producing an ELF static archive)
//! turn `std` off and supply their own `#[global_allocator]` and
//! `#[panic_handler]`. The Rust target should still be `x86_64-unknown-uefi`
//! when std is on (the only target that ships uefi_std); for the no_std
//! variant `x86_64-unknown-none` is the natural choice and is what the GRUB
//! module path uses.

extern crate alloc;

// Internal: the C runtime surface flite's archive links against. These modules
// define `#[no_mangle]` symbols; they have no public Rust API.
mod cprintf;
mod shim;

pub mod wav;

use alloc::ffi::CString;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};

/// Mirror of flite's `cst_wave` (include/cst_wave.h). Public so [`wav`] can use
/// it; you normally interact with [`Wave`] instead.
#[repr(C)]
pub struct CstWave {
    pub type_: *const c_char,
    pub sample_rate: c_int,
    pub num_samples: c_int,
    pub num_channels: c_int,
    pub samples: *mut i16,
}

extern "C" {
    fn flite_init() -> c_int;
    fn register_cmu_us_slt(voxdir: *const c_char) -> *mut c_void;
    fn flite_text_to_wave(text: *const c_char, voice: *mut c_void) -> *mut CstWave;
    fn delete_wave(w: *mut CstWave);
    fn feat_set_float(f: *mut c_void, name: *const c_char, v: f32);
}

/// Mirror of the head of flite's `cst_voice_struct` (include/cst_voice.h). Only
/// the first two fields are accessed; the rest of the struct is opaque.
#[repr(C)]
struct CstVoice {
    _name: *const c_char,
    features: *mut c_void,
}

/// A registered flite voice. Obtain one with [`init`].
pub struct Voice {
    ptr: *mut c_void,
}

/// Initialize flite and register the compiled-in `cmu_us_slt` US-English
/// clustergen voice. Returns `None` if voice registration fails. Safe to treat
/// as a one-time setup call.
pub fn init() -> Option<Voice> {
    unsafe {
        flite_init();
        let ptr = register_cmu_us_slt(core::ptr::null());
        if ptr.is_null() {
            None
        } else {
            Some(Voice { ptr })
        }
    }
}

impl Voice {
    /// Set flite's `duration_stretch` feature on the voice. Values < 1.0 speak
    /// faster, > 1.0 slower; default is 1.0. Pitch is preserved (flite stretches
    /// in time, not in sample rate).
    pub fn set_duration_stretch(&self, stretch: f32) {
        unsafe {
            let v = self.ptr as *mut CstVoice;
            if v.is_null() {
                return;
            }
            let features = (*v).features;
            if features.is_null() {
                return;
            }
            let name = b"duration_stretch\0".as_ptr() as *const c_char;
            feat_set_float(features, name, stretch);
        }
    }

    /// Synthesize `text` to PCM audio. Returns `None` if `text` contains an
    /// interior NUL byte or flite produces no wave.
    pub fn synthesize(&self, text: &str) -> Option<Wave> {
        let c = CString::new(text).ok()?;
        let w = unsafe { flite_text_to_wave(c.as_ptr(), self.ptr) };
        if w.is_null() {
            None
        } else {
            Some(Wave { ptr: w })
        }
    }
}

/// Synthesized audio: 16-bit signed PCM. Owns the underlying flite wave and
/// frees it on drop.
pub struct Wave {
    ptr: *mut CstWave,
}

impl Wave {
    #[inline]
    fn inner(&self) -> &CstWave {
        unsafe { &*self.ptr }
    }

    /// Sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.inner().sample_rate.max(0) as u32
    }

    /// Number of interleaved channels (1 for the built-in mono voice).
    pub fn num_channels(&self) -> u32 {
        self.inner().num_channels.max(0) as u32
    }

    /// The interleaved 16-bit signed PCM samples
    /// (length = `num_samples * num_channels`). Valid for the lifetime of this
    /// `Wave`. Feed these to an audio driver.
    pub fn samples(&self) -> &[i16] {
        let w = self.inner();
        let n = (w.num_samples.max(0) as usize) * (w.num_channels.max(1) as usize);
        if w.samples.is_null() || n == 0 {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(w.samples, n) }
        }
    }

    /// Serialize to a canonical little-endian 16-bit PCM WAV file (RIFF/WAVE).
    pub fn to_wav(&self) -> Vec<u8> {
        unsafe { wav::wave_to_wav(self.inner()) }
    }
}

impl Drop for Wave {
    fn drop(&mut self) {
        unsafe { delete_wave(self.ptr) }
    }
}
