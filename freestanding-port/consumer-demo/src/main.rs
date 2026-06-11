//! Your application: depend on `flite-freestanding`, synthesize to PCM, then hand
//! the samples to your audio driver. Build for a freestanding target with nightly:
//!
//!   cargo build --target x86_64-unknown-uefi
//!
//! (No `#![feature(...)]` needed here — that's internal to the library.)

fn main() {
    let voice = flite_freestanding::init().expect("flite voice registration failed");
    let wave = voice.synthesize("hello from a separate binary").expect("synthesis failed");

    let samples: &[i16] = wave.samples();
    let rate = wave.sample_rate();
    let channels = wave.num_channels();
    println!(
        "consumer-demo: {} samples @ {} Hz x{} channels",
        samples.len(), rate, channels
    );

    // >>> Hand `samples` to your audio driver here, e.g.:
    //     my_audio_driver.play(samples, rate, channels);
    // `samples` is interleaved 16-bit signed PCM, valid while `wave` is alive.

    println!("consumer-demo: done");
}
