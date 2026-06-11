# Golden reference output

`hello.wav` is the exact WAV produced by the `synth` EFI app when it synthesizes
`"hello world"` under QEMU + OVMF. The `UEFI synth (OVMF)` GitHub workflow
(`.github/workflows/uefi-synth-ovmf.yml`) boots the app and asserts its output is
**byte-for-byte identical** to this file.

- 16-bit signed PCM, 16 kHz, mono
- 19760 samples, 39564 bytes (RIFF/WAVE)
- sha256: `f2d25e7dec656b15661bb434fff5316453d7035834ec4dd1308ead2198dc675f`

## Why it's reproducible

Synthesis is deterministic: flite's `rand()` is a fixed-seed LCG, and all
floating-point math runs through the pinned `libm` crate (`Cargo.lock`) plus
IEEE-exact soft-float libcalls from `compiler_builtins`. clang/LLVM only emits
those libcalls in source order, so the bytes are stable across toolchain
versions.

## Regenerating

If a deliberate change alters the output (different voice, phrase, sample
format, …), regenerate from `freestanding-port/`:

```bash
make qemu
cp esp/EFI/BOOT/hello.wav tests/golden/hello.wav
```

Then commit the new `hello.wav`.
