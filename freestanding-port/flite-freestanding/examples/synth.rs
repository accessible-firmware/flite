//! Demo: register the built-in voice, synthesize a line, and write the WAV to
//! the EFI System Partition. Run under QEMU+OVMF via `make -C freestanding-port qemu`.
//!
//!   cargo build --example synth --target x86_64-unknown-uefi

fn main() {
    println!("FLITE-UEFI: start");
    let voice = match flite_freestanding::init() {
        Some(v) => v,
        None => {
            println!("FAIL: voice registration");
            return;
        }
    };
    let wave = match voice.synthesize("hello world") {
        Some(w) => w,
        None => {
            println!("FAIL: synthesis");
            return;
        }
    };
    println!(
        "FLITE-UEFI: num_samples={} sample_rate={} num_channels={}",
        wave.samples().len(),
        wave.sample_rate(),
        wave.num_channels()
    );
    println!("FLITE-UEFI: SYNTHESIS OK");

    let buf = wave.to_wav();
    println!("FLITE-UEFI: wav bytes={}", buf.len());
    write_wav(&buf);

    // Return to the firmware boot manager (do not spin).
    println!("FLITE-UEFI: done — returning control to firmware");
}

/// Write the WAV to the ESP and read it back to confirm.
fn write_wav(buf: &[u8]) {
    for path in ["hello.wav", "\\hello.wav", "/hello.wav"] {
        match std::fs::write(path, buf) {
            Ok(()) => {
                println!("FLITE-UEFI: wrote {:?} ({} bytes)", path, buf.len());
                if let Ok(rb) = std::fs::read(path) {
                    let magic = &rb[..rb.len().min(4)];
                    println!(
                        "FLITE-UEFI: readback {:?} ok, {} bytes, magic={:?}",
                        path, rb.len(), core::str::from_utf8(magic).unwrap_or("?")
                    );
                    println!("FLITE-UEFI: WAV WRITE OK");
                    return;
                }
            }
            Err(e) => println!("FLITE-UEFI: write {:?} failed: {}", path, e),
        }
    }
    println!("FLITE-UEFI: WAV WRITE FAILED (all paths)");
}
