#!/usr/bin/env bash
# Boot the flite-freestanding `synth` EFI app under QEMU + OVMF, confirm it
# synthesizes and writes hello.wav from inside the UEFI firmware environment,
# and (unless disabled) check the result is bit-identical to the golden WAV.
#
# Usage:
#   tests/run-qemu-test.sh [HDA_CONTROLLER]
#
# HDA_CONTROLLER is the QEMU Intel HDA controller to attach (default intel-hda;
# the CI matrix also runs ich9-intel-hda). The codec is attached but unused —
# the app's output is a WAV file; this wires up the device for later audio work.
#
# Overridable via environment:
#   EFI_APP       path to synth.efi (default: the crate's debug build output)
#   GOLDEN        golden WAV to compare against (default: tests/golden/hello.wav;
#                 set GOLDEN= empty to skip the bit-identical comparison)
#   OVMF_CODE     OVMF code image  (auto-detected across distros if unset)
#   OVMF_VARS     OVMF vars image  (auto-detected across distros if unset)
#   QEMU_TIMEOUT  seconds before QEMU is stopped (default 90)
set -euo pipefail

HDA="${1:-intel-hda}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FS_DIR="$(dirname "$HERE")"

EFI_APP="${EFI_APP:-$FS_DIR/flite-freestanding/target/x86_64-unknown-uefi/debug/examples/synth.efi}"
GOLDEN="${GOLDEN-$FS_DIR/tests/golden/hello.wav}"
QEMU_TIMEOUT="${QEMU_TIMEOUT:-90}"
ESP_DIR="$FS_DIR/esp"
LOG="$FS_DIR/qemu-$HDA.log"

die() { echo "::error::$*" >&2; exit 1; }

# --- locate firmware --------------------------------------------------------
pick() { for c in "$@"; do [ -f "$c" ] && { echo "$c"; return; }; done; }
OVMF_CODE="${OVMF_CODE:-$(pick \
  /usr/share/OVMF/OVMF_CODE_4M.fd \
  /usr/share/OVMF/OVMF_CODE.fd \
  /usr/share/edk2/x64/OVMF_CODE.4m.fd \
  /usr/share/edk2-ovmf/x64/OVMF_CODE.fd)}"
OVMF_VARS="${OVMF_VARS:-$(pick \
  /usr/share/OVMF/OVMF_VARS_4M.fd \
  /usr/share/OVMF/OVMF_VARS.fd \
  /usr/share/edk2/x64/OVMF_VARS.4m.fd \
  /usr/share/edk2-ovmf/x64/OVMF_VARS.fd)}"

[ -f "$EFI_APP" ]   || die "synth.efi not found at $EFI_APP (run 'make synth' first)"
[ -n "$OVMF_CODE" ] || die "could not locate OVMF_CODE.fd — set OVMF_CODE"
[ -n "$OVMF_VARS" ] || die "could not locate OVMF_VARS.fd — set OVMF_VARS"

# --- stage the ESP ----------------------------------------------------------
rm -rf "$ESP_DIR"
mkdir -p "$ESP_DIR/EFI/BOOT"
cp "$EFI_APP" "$ESP_DIR/EFI/BOOT/BOOTX64.EFI"
VARS_RW="$(mktemp)"
cp "$OVMF_VARS" "$VARS_RW"
trap 'rm -f "$VARS_RW"' EXIT

# --- boot -------------------------------------------------------------------
# The app writes hello.wav then returns to the firmware boot menu, which sits
# idle — so QEMU is stopped by `timeout`; success is decided by the serial log
# and the produced file, not QEMU's exit code.
echo "=== booting QEMU q35 with -device $HDA (timeout ${QEMU_TIMEOUT}s) ==="
timeout "$QEMU_TIMEOUT" qemu-system-x86_64 \
  -machine q35 -m 256 -nographic \
  -drive if=pflash,format=raw,unit=0,readonly=on,file="$OVMF_CODE" \
  -drive if=pflash,format=raw,unit=1,file="$VARS_RW" \
  -drive format=raw,file=fat:rw:"$ESP_DIR" \
  -audiodev none,id=snd0 \
  -device "$HDA",id=hda0 \
  -device hda-output,bus=hda0.0,audiodev=snd0 \
  -net none > "$LOG" 2>&1 || true

echo "--- serial log (FLITE-UEFI lines) ---"
grep -a "FLITE-UEFI" "$LOG" || true
echo "-------------------------------------"

# --- verify -----------------------------------------------------------------
grep -aq "FLITE-UEFI: SYNTHESIS OK" "$LOG" || die "synthesis did not complete"
grep -aq "FLITE-UEFI: WAV WRITE OK" "$LOG" || die "WAV was not written"
WAV="$ESP_DIR/EFI/BOOT/hello.wav"
[ -f "$WAV" ] || die "hello.wav missing on the ESP after the run"

if [ -n "$GOLDEN" ]; then
  if ! cmp "$WAV" "$GOLDEN"; then
    echo "produced: $(sha256sum "$WAV")"
    echo "golden:   $(sha256sum "$GOLDEN")"
    die "produced hello.wav differs from golden reference ($GOLDEN)"
  fi
  echo "PASS [$HDA]: hello.wav written and bit-identical to golden reference"
else
  echo "PASS [$HDA]: hello.wav written ($(stat -c%s "$WAV") bytes); golden comparison skipped"
fi
