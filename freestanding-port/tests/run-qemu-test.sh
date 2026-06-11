#!/usr/bin/env bash
# Boot the flite-freestanding `synth` EFI app under QEMU + OVMF, confirm it
# synthesizes and writes hello.wav from inside the UEFI firmware environment,
# and (unless disabled) check the result is bit-identical to the golden WAV.
#
# Usage:
#   tests/run-qemu-test.sh
#
# The ESP is a real FAT image driven via mtools, NOT QEMU's `fat:rw:` directory
# backend: VVFAT's write-back of newly created files to the host is unreliable
# and version-dependent (it silently dropped hello.wav on CI's QEMU). A raw FAT
# image stores guest block writes verbatim, so we read hello.wav straight back
# out with `mcopy` after the run.
#
# Requires: qemu-system-x86_64, OVMF, mtools (mformat/mmd/mcopy).
# Overridable via environment:
#   EFI_APP       path to synth.efi (default: the crate's debug build output)
#   GOLDEN        golden WAV to compare against (default: tests/golden/hello.wav;
#                 set GOLDEN= empty to skip the bit-identical comparison)
#   OVMF_CODE     OVMF code image  (auto-detected across distros if unset)
#   OVMF_VARS     OVMF vars image  (auto-detected across distros if unset)
#   QEMU_TIMEOUT  seconds before QEMU is stopped (default 90)
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FS_DIR="$(dirname "$HERE")"

EFI_APP="${EFI_APP:-$FS_DIR/flite-freestanding/target/x86_64-unknown-uefi/debug/examples/synth.efi}"
GOLDEN="${GOLDEN-$FS_DIR/tests/golden/hello.wav}"
QEMU_TIMEOUT="${QEMU_TIMEOUT:-90}"
ESP_IMG="$FS_DIR/esp.img"
OUT_WAV="$FS_DIR/hello.wav"
LOG="$FS_DIR/qemu.log"

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
command -v mformat >/dev/null || die "mtools not found (install 'mtools')"

# --- build the ESP image ----------------------------------------------------
# 64 MiB FAT32 (the usual ESP layout): EFI/BOOT/BOOTX64.EFI is the removable-
# media default boot path, so OVMF runs it automatically at power-on.
rm -f "$ESP_IMG" "$OUT_WAV"
dd if=/dev/zero of="$ESP_IMG" bs=1M count=64 status=none
mformat -i "$ESP_IMG" -F ::
mmd -i "$ESP_IMG" ::/EFI ::/EFI/BOOT
mcopy -i "$ESP_IMG" "$EFI_APP" ::/EFI/BOOT/BOOTX64.EFI

VARS_RW="$(mktemp)"
cp "$OVMF_VARS" "$VARS_RW"
trap 'rm -f "$VARS_RW"' EXIT

# --- boot -------------------------------------------------------------------
# The app writes hello.wav then returns to the firmware boot menu, which sits
# idle — so QEMU is stopped by `timeout`; success is decided by the serial log
# and the file extracted from the image, not QEMU's exit code. The firmware
# flushes the file to the block device when the app closes it, so it is present
# in the image well before the timeout fires.
echo "=== booting QEMU q35 (timeout ${QEMU_TIMEOUT}s) ==="
timeout "$QEMU_TIMEOUT" qemu-system-x86_64 \
  -machine q35 -m 256 -nographic \
  -drive if=pflash,format=raw,unit=0,readonly=on,file="$OVMF_CODE" \
  -drive if=pflash,format=raw,unit=1,file="$VARS_RW" \
  -drive format=raw,file="$ESP_IMG" \
  -net none > "$LOG" 2>&1 || true

echo "--- serial log (FLITE-UEFI lines) ---"
grep -a "FLITE-UEFI" "$LOG" || true
echo "-------------------------------------"

# --- verify -----------------------------------------------------------------
grep -aq "FLITE-UEFI: SYNTHESIS OK" "$LOG" || die "synthesis did not complete"
grep -aq "FLITE-UEFI: WAV WRITE OK" "$LOG" || die "WAV was not written"

# Pull the file the app wrote back out of the image.
mcopy -i "$ESP_IMG" ::/EFI/BOOT/hello.wav "$OUT_WAV" 2>/dev/null \
  || die "hello.wav not found in the ESP image after the run"

if [ -n "$GOLDEN" ]; then
  if ! cmp "$OUT_WAV" "$GOLDEN"; then
    echo "produced: $(sha256sum "$OUT_WAV")"
    echo "golden:   $(sha256sum "$GOLDEN")"
    die "produced hello.wav differs from golden reference ($GOLDEN)"
  fi
  echo "PASS: hello.wav written and bit-identical to golden reference"
else
  echo "PASS: hello.wav written ($(stat -c%s "$OUT_WAV") bytes); golden comparison skipped"
fi
