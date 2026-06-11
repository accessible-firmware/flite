# Flite for freestanding x86_64

This directory builds [flite](https://github.com/festvox/flite) into a **Rust library** (`flite-freestanding`) that runs the CMU flite TTS engine on **freestanding x86_64 targets** — with no operating system underneath.
It works anywhere you can provide an allocator and a panic handler: UEFI firmware (`x86_64-unknown-uefi`), a GRUB module, or any bare-metal/`x86_64-unknown-none` environment.
Your own binary depends on the library, synthesizes text to PCM, and feeds the samples to its audio driver.
This is a proof-of-concept and **not** something intended for upstream flite; therefore this is more-or-less a hard fork.

## What it does

`flite-freestanding` is a library crate that registers the compiled-in `cmu_us_slt`
clustergen voice and synthesizes text to 16-bit 16 kHz mono PCM.
It is `no_std` + `alloc` by default; the consumer supplies a `#[global_allocator]`
and `#[panic_handler]` (the optional `std` feature wires those up for the UEFI-std
build path). Two binaries demonstrate it:

- `examples/synth.rs` writes `hello.wav` to the ESP, used by the QEMU test, and
- `consumer-demo/`: a crate showing how to depend on the library from your own app. Demonstrated under QEMU + OVMF.

## Using the library

Depend on it by path and call the small safe API:

```toml
# your-app/Cargo.toml
[dependencies]
flite-freestanding = { path = "../path/to/freestanding-port/flite-freestanding" }
[profile.dev]
panic = "abort"
[profile.release]
panic = "abort"
```

```rust
let voice = flite_freestanding::init().expect("voice registration");
let wave  = voice.synthesize("hello world").expect("synthesis");
let pcm: &[i16] = wave.samples();       // interleaved 16-bit signed PCM
let rate        = wave.sample_rate();    // Hz
let channels    = wave.num_channels();
// hand `pcm` to your audio driver; or `wave.to_wav()` for RIFF/WAVE bytes.
```

Build your app with nightly for a freestanding target, e.g. UEFI:

```bash
cargo build --target x86_64-unknown-uefi
```

or a bare ELF target (GRUB module, bare metal):

```bash
cargo build --target x86_64-unknown-none --no-default-features
```

The matching flite C archive must exist next to the library crate
(`libflite_coff.a` for the UEFI/COFF target, `libflite_elf.a` for ELF targets);
`build.rs` picks the right one from the target triple and finds it relative to the
crate, so it works as a dependency.

## How it works

This **cross-compiles flite's C directly for the freestanding target** and supplies
the C runtime from Rust:

- clang's `x86_64-unknown-uefi` triple emits COFF objects; `x86_64-unknown-none` emits ELF. Either set links directly with Rust's output for the matching target as they share the same ABI. [Note that soft-float is a hard requirement](https://github.com/rust-lang/rust/blob/main/compiler/rustc_target/src/spec/targets/x86_64_unknown_uefi.rs#L19)
- `compiler-builtins` provides `memcpy`/`memmove`/`memset`.
- `long` is 32-bit on this target; the printf shim accounts for that.
- The voice is compiled in as `const` data, so synthesis touches no files. The stdio/file/mmap symbols are link-only stubs; they should probably be removed at some point.

## Prerequisites

- **clang / LLVM 21+** (needs a *working* `x86_64-unknown-uefi` target; check with `clang --target=x86_64-unknown-uefi -c -x c /dev/null -o /dev/null`), plus `llvm-ar`. LLVM 19/20 *recognize* the triple but emit a COFF datalayout the backend rejects (`backend data layout 'e-m:w-…' does not match … 'e-m:e-…'`); the fix (llvm/llvm-project#120632, #127290) first shipped in LLVM 21.
- **Rust nightly** with the UEFI target: `rustup toolchain install nightly && rustup target add --toolchain nightly x86_64-unknown-uefi` (the printf shim uses the nightly `c_variadic` feature).
- **QEMU** (`qemu-system-x86_64`) and **OVMF** firmware. On Arch: `pacman -S qemu-full edk2-ovmf`. Override the OVMF paths for your distro, e.g.  Debian/Ubuntu: `make qemu OVMF_CODE=/usr/share/OVMF/OVMF_CODE.fd OVMF_VARS=/usr/share/OVMF/OVMF_VARS.fd`.
- `python3` (the `coff`/`elf` targets use it to read `compile_commands.json`).

## Build & run

Everything is driven by `freestanding-port/Makefile`. Run the targets from `freestanding-port/`
(or from anywhere with `make -C freestanding-port <target>`); `make help` lists them all.

```bash
cd freestanding-port

# 1. Build flite natively once (produces the static libs used by the native and lets you regenerate compile_commands.json if needed).
make native

# 2. Optional: check the C library + voice on the host.
make native-test

# 3. (UEFI) Cross-compile flite's C to the COFF archive for UEFI.
make coff

# 3. (Freestanding) Cross-compile flite's C to the ELF archive for freestanding/GRUB/boot module initialization.
make elf

# 4. Build the demo EFI application (the `synth` example of the library).
make synth

# 5. Run it under QEMU + OVMF (prints to the console, writes hello.wav).
make qemu
```

To clean up: `make clean` removes all build intermediates and run artifacts; `make distclean` runs `make clean` _and_ removes the generated archives.

Example console output of `make qemu`:

```
FLITE-UEFI: num_samples=19760 sample_rate=16000 num_channels=1
FLITE-UEFI: SYNTHESIS OK
FLITE-UEFI: wrote "hello.wav" (39564 bytes)
FLITE-UEFI: readback "hello.wav" ok, 39564 bytes, magic="RIFF"
FLITE-UEFI: WAV WRITE OK
```

`make qemu` uses QEMU's `fat:rw:` to back the ESP with `freestanding-port/esp/`, so after the run the produced file is on the host at `freestanding-port/esp/EFI/BOOT/hello.wav`.

## Limitations / notes

- Only `cmu_us_slt` (a US-English clustergen voice) is compiled in; one phrase is synthesized. Other voices/languages would each be linked in similarly.
- No audio playback (UEFI has no standard audio protocol) — output is PCM/WAV.
- Sample counts differ slightly from a host build because the shim's `rand()` and `libm` differ from glibc, nudging the clustergen duration model. Both are valid speech.
- `cargo build` requires the matching archive to exist next to the crate — `freestanding-port/libflite_coff.a` for the UEFI target, `freestanding-port/libflite_elf.a` for ELF targets (run step 3 first if you remove the checked-in copy).
