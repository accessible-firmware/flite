# Developer Guide

This describes the process of updating, patching, and upstreaming new features into this version of `flite/`.

## Add a voice/featureset already supported by Flite

1. Add the voice/feature's source directory to the `SELECTION` map in
   `freestanding-port/tools/gen_compile_commands.py` (e.g. `"lang/cmu_us_awb": "all"`),
   then `make compile-commands` to regenerate the source list.
2. `make coff && make elf` (make for both x86 UEFI and x86 freestanding)
3. Determine if there is a new symbol to fill in by running: `TODO`
4. If there is, add it to `flite-freestanding/src/shim.rs` and to `ueffi-undefined-symbols.txt`

## Regenerating `compile_commands.json`

`freestanding-port/compile_commands.json` is a **generated artifact**, not a
source of truth — it lists which flite C files the cross-build compiles, with
absolute paths for *this* checkout. The source of truth is the `SELECTION` map in
`tools/gen_compile_commands.py`.

- Regenerate it any time the source set changes: `make compile-commands`.
- The cross-builds (`make coff`/`make elf`) also auto-generate it if it's missing,
  so a fresh checkout builds without a committed copy.
- Because it carries machine-local absolute paths, **you don't need to commit it**
  (CI regenerates its own — see `.github/workflows/uefi-synth-ovmf.yml`). If it's
  still tracked, `git rm --cached freestanding-port/compile_commands.json` to stop
  tracking it; it's covered by `.gitignore`.
- Only the `file` entries matter to the build; the `command` field is filler for
  editor tooling. The real compile flags come from `COFF_FLAGS`/`ELF_FLAGS` in the
  `Makefile` (which is where `-DDIE_ON_ERROR`, `-DCST_AUDIO_NONE`, soft-float, etc.
  actually live).

## Run the QEMU/OVMF test

`make qemu-test` boots `synth.efi` under QEMU + OVMF and asserts it writes a
`hello.wav` that is bit-identical to `tests/golden/hello.wav` (override the HDA
controller with `HDA=ich9-intel-hda`). This is the same check CI runs via
`tests/run-qemu-test.sh` across both Intel HDA controllers. If a deliberate
change alters the output, regenerate the golden file (see `tests/golden/README.md`).

## Expose a new C-compatible API

1. Declare the C funciton in the `extern "C"` block in `flite-freestanding/src/lib.rs`
2. Add a save wrapper mothed that executes on `CstWave`, `CstVoice` or some other flite-derived struct.

## Add a missing `libc`-derived function

1. Add `#[no_mangle] pub unsafe extern "C" fn <name>(...) -> ...` in `shim.rs`
2. Add its declaration to the `cinclude/*.h` with the appropriate header namme filled in.
3. *NOTE: do not define `memcpy/memmove/memset` as they are defined by the `compiler_builtins` module available on all Rust targets.

## Fix a missing symbol issue

1. Run `llvm-nm libflite_coff.a | grep <sym>` to see if it's `U` (undefined), `T` (defined).
2. Compare with `uefi-undefined-symbols.txt`.

## Integrate it with an audio driver to use speakers for synthesis

See our current freestanding audio implementations:

- Intel HDA: https://github.com/accessible-firmware/hda_freestanding/
- ...more coming soon.
