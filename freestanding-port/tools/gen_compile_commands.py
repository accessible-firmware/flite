#!/usr/bin/env python3
"""Generate compile_commands.json for the freestanding cross-build.

The cross-builds (`make coff` / `make elf`) read only the `file` entries and
compile each with their own flags, so this only needs to settle the right *set*
of flite C sources. That set is the configured-for-freestanding subset of the
flite library: the relevant `src/` modules plus the three languages we link in
(usenglish, cmulex, cmu_us_slt), minus the platform/socket/raw-data files the
freestanding build never wants, plus our own math_bridge.c.

It enumerates the source tree directly (no native build, no `bear`, no
`configure`) so it is fast and produces identical results everywhere — the
output just carries this checkout's absolute paths, which is the whole point:
the file is a generated artifact and need not be committed.

Each directory below is either fully included, included except a documented
blocklist, or restricted to an allowlist. Keep these in sync if flite's sources
change. Usage:

  gen_compile_commands.py <repo-root> <out.json> <math_bridge.c>
"""
import glob
import json
import os
import sys

# dir (repo-relative) -> selection rule:
#   "all"                  : every *.c in the dir
#   ("only", {names})      : only these files (allowlist)
#   ("except", {names})    : every *.c except these (blocklist)
SELECTION = {
    # Core engine modules — taken whole.
    "src/cg": "all",
    "src/hrg": "all",
    "src/lexicon": "all",
    "src/regex": "all",
    "src/speech": "all",
    "src/stats": "all",
    "src/synth": "all",
    "src/wavesynth": "all",
    # Audio: only the generic core + the null driver. The platform drivers
    # (alsa/oss/pulse/sun/win/wince/palmos/command) and the socket
    # client/server are unused under -DCST_AUDIO_NONE / -DCST_NO_SOCKETS.
    "src/audio": ("only", {"audio.c", "au_none.c", "au_streaming.c"}),
    # Utils: everything except platform file/mmap variants and the socket
    # helper. The freestanding build uses the stdio/posix variants + shims.
    "src/utils": ("except", {
        "cst_file_palmos.c", "cst_file_wince.c",
        "cst_mmap_none.c", "cst_mmap_win32.c",
        "cst_socket.c",
    }),
    # Languages we compile in.
    "lang/usenglish": "all",
    # cmulex: drop the uncompressed/raw data and the huff-table generator
    # inputs; the build links the compressed cmu_lex_data.c.
    "lang/cmulex": ("except", {
        "cmu_lex_data_raw.c",
        "cmu_lex_entries_huff_table.c",
        "cmu_lex_num_bytes.c",
        "cmu_lex_phones_huff_table.c",
    }),
    "lang/cmu_us_slt": "all",
}


def select(dir_abs, rule):
    names = sorted(os.path.basename(p) for p in glob.glob(os.path.join(dir_abs, "*.c")))
    if rule == "all":
        return names
    kind, listed = rule
    if kind == "only":
        missing = listed - set(names)
        if missing:
            raise SystemExit(f"gen_compile_commands: {dir_abs}: allowlisted but absent: {sorted(missing)}")
        return [n for n in names if n in listed]
    if kind == "except":
        return [n for n in names if n not in listed]
    raise SystemExit(f"gen_compile_commands: bad rule {rule!r}")


def main():
    repo_root, out_path, math_bridge = sys.argv[1:4]
    repo_root = os.path.abspath(repo_root)

    files = []
    for rel_dir, rule in SELECTION.items():
        dir_abs = os.path.join(repo_root, rel_dir)
        if not os.path.isdir(dir_abs):
            raise SystemExit(f"gen_compile_commands: missing source dir {dir_abs}")
        for name in select(dir_abs, rule):
            files.append(os.path.join(dir_abs, name))

    # Our soft-float bridge — part of the freestanding build, lives outside flite.
    files.append(os.path.abspath(math_bridge))

    entries = [
        {
            # Representative command; the cross-build supplies real flags. Kept
            # populated so editor tooling (clangd) can still parse the file.
            "command": f"cc -I{repo_root}/include -c {f}",
            "directory": os.path.dirname(f),
            "file": f,
        }
        for f in sorted(files)
    ]
    with open(out_path, "w") as fh:
        json.dump(entries, fh, indent=2)
        fh.write("\n")
    print(f"gen_compile_commands: wrote {len(entries)} entries to {out_path}")


if __name__ == "__main__":
    main()
