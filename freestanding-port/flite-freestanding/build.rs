use std::env;

fn main() {
    // The cross-compiled flite C archive sits one directory up
    // (`freestanding-port/`). We ship two variants because clang's output format
    // depends on the target triple it was invoked with:
    //
    //   - libflite_coff.a    — COFF (PE+ x86_64). Used when the consuming
    //                          Rust crate targets x86_64-unknown-uefi
    //                          (the original standalone EFI app path).
    //   - libflite_elf.a     — ELF. Used when the consumer is e.g. a GRUB
    //                          module that needs to statically archive
    //                          this crate's output into a relocatable
    //                          ELF, and therefore has to target
    //                          x86_64-unknown-none (the only stable
    //                          freestanding ELF target Rust ships).
    //
    // Either archive contains the same compiled flite source set plus
    // `math_bridge.c`; the only difference is the object file format.
    // Regenerate them with:
    //   make -C freestanding-port coff  -> libflite_coff.a   (COFF)
    //   make -C freestanding-port elf   -> libflite_elf.a    (ELF)
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target = env::var("TARGET").unwrap_or_default();
    let dir = format!("{manifest}/..");

    let (lib_name, lib_filename) = if target == "x86_64-unknown-uefi" {
        ("flite_coff", "libflite_coff.a")
    } else {
        // Everything else (x86_64-unknown-none, x86_64-unknown-linux-gnu
        // for host-side native testing, ...) wants the ELF archive. We
        // don't gate further on the OS field because clang emits ELF for
        // every non-UEFI x86_64 target we plausibly compile this crate
        // against, and the consumer has explicitly chosen a non-UEFI
        // target by virtue of needing ELF output.
        ("flite_elf", "libflite_elf.a")
    };

    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-lib=static={lib_name}");
    println!("cargo:rerun-if-changed={dir}/{lib_filename}");
    println!("cargo:rerun-if-env-changed=TARGET");
}
