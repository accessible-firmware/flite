//! Rust implementation of the C runtime/libc surface that flite's compiled C
//! archive references. These are linked against `libflite_coff.a` / `libflite_elf.a`.
//!
//! Calling convention: on x86_64-unknown-uefi the C ABI is Win64, which matches
//! Rust `extern "C"` on this target, so `#[no_mangle] extern "C"` lines up with
//! the COFF symbols emitted by the cross-compiled flite objects.

// The alloc crate's `alloc` module contains the allocator functions
// (alloc/dealloc/realloc); `Layout` lives in core::alloc. We import the
// functions under aliases because `alloc::alloc` as a module path would
// shadow the crate name `alloc` and confuse subsequent paths.
use alloc::alloc::{alloc as heap_alloc, dealloc as heap_dealloc, realloc as heap_realloc};
use core::alloc::Layout;
use core::ffi::{c_char, c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------
//
// We prepend a 16-byte header to every allocation. The header stores the total
// allocated size (including the header) so that `free`/`realloc` can recover
// the layout without the caller passing a size. We always allocate at align 16.

const HEADER: usize = 16;
const ALIGN: usize = 16;

#[inline]
fn layout_for(total: usize) -> Layout {
    Layout::from_size_align(total, ALIGN).unwrap()
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let total = match size.checked_add(HEADER) {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };
    let base = heap_alloc(layout_for(total));
    if base.is_null() {
        return core::ptr::null_mut();
    }
    *(base as *mut usize) = total;
    base.add(HEADER) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let bytes = match nmemb.checked_mul(size) {
        Some(b) => b,
        None => return core::ptr::null_mut(),
    };
    if bytes == 0 {
        return core::ptr::null_mut();
    }
    let p = malloc(bytes);
    if !p.is_null() {
        core::ptr::write_bytes(p as *mut u8, 0, bytes);
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }
    let base = (ptr as *mut u8).sub(HEADER);
    let old_total = *(base as *mut usize);
    let new_total = match size.checked_add(HEADER) {
        Some(t) => t,
        None => return core::ptr::null_mut(),
    };
    let new_base = heap_realloc(base, layout_for(old_total), new_total);
    if new_base.is_null() {
        return core::ptr::null_mut();
    }
    *(new_base as *mut usize) = new_total;
    new_base.add(HEADER) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let base = (ptr as *mut u8).sub(HEADER);
    let total = *(base as *mut usize);
    heap_dealloc(base, layout_for(total));
}

// ---------------------------------------------------------------------------
// mem / str
// ---------------------------------------------------------------------------

// NOTE: memcpy / memmove / memset are intentionally NOT defined here.
// `compiler-builtins` (linked into every Rust program) already provides correct
// C-callable `memcpy`/`memmove`/`memset`/`memcmp`/`strlen` for freestanding
// targets like x86_64-unknown-uefi. Defining our own from
// `core::ptr::copy_nonoverlapping`/`write_bytes` is a trap: those intrinsics
// lower *back* into calls to `memcpy`/`memset`, so our versions recursed
// infinitely and overflowed the stack during std startup. Let compiler-builtins
// own them; flite's C archive links against those.

#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

#[no_mangle]
pub unsafe extern "C" fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb {
            return ca as c_int - cb as c_int;
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut i = 0usize;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    let mut i = 0usize;
    // copy up to the NUL
    while i < n {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    // pad remainder with NUL
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    let start = strlen(dst);
    strcpy(dst.add(start), src);
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8 as c_char;
    let mut i = 0usize;
    loop {
        let cur = *s.add(i);
        if cur == target {
            return s.add(i) as *mut c_char;
        }
        if cur == 0 {
            return core::ptr::null_mut();
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8 as c_char;
    let mut last: *mut c_char = core::ptr::null_mut();
    let mut i = 0usize;
    loop {
        let cur = *s.add(i);
        if cur == target {
            last = s.add(i) as *mut c_char;
        }
        if cur == 0 {
            return last;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char {
    let nlen = strlen(needle);
    if nlen == 0 {
        return haystack as *mut c_char;
    }
    let mut i = 0usize;
    loop {
        // ensure haystack[i] exists / not past NUL
        if *haystack.add(i) == 0 {
            return core::ptr::null_mut();
        }
        if strncmp(haystack.add(i), needle, nlen) == 0 {
            return haystack.add(i) as *mut c_char;
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// wide
// ---------------------------------------------------------------------------
//
// flite is built with -fshort-wchar so wchar_t is 16-bit.

#[no_mangle]
pub unsafe extern "C" fn wcslen(s: *const u16) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

// ---------------------------------------------------------------------------
// ctype
// ---------------------------------------------------------------------------

// These mirror C's `<ctype.h>`: the argument is an `int` that must be either
// EOF (-1) or representable as `unsigned char`. We narrow to `u8` and defer to
// core's `is_ascii_*`/`to_ascii_*` rather than hand-coding the ranges; values
// outside the `u8` range (e.g. EOF) classify as false / pass through unchanged,
// matching the C contract.

#[no_mangle]
pub extern "C" fn isalnum(c: c_int) -> c_int {
    u8::try_from(c).is_ok_and(|b| b.is_ascii_alphanumeric()) as c_int
}

#[no_mangle]
pub extern "C" fn islower(c: c_int) -> c_int {
    u8::try_from(c).is_ok_and(|b| b.is_ascii_lowercase()) as c_int
}

#[no_mangle]
pub extern "C" fn isupper(c: c_int) -> c_int {
    u8::try_from(c).is_ok_and(|b| b.is_ascii_uppercase()) as c_int
}

#[no_mangle]
pub extern "C" fn tolower(c: c_int) -> c_int {
    match u8::try_from(c) {
        Ok(b) => b.to_ascii_lowercase() as c_int,
        Err(_) => c,
    }
}

#[no_mangle]
pub extern "C" fn toupper(c: c_int) -> c_int {
    match u8::try_from(c) {
        Ok(b) => b.to_ascii_uppercase() as c_int,
        Err(_) => c,
    }
}

// ---------------------------------------------------------------------------
// math (via libm) — f64 bridge
// ---------------------------------------------------------------------------
//
// flite is compiled with soft-float (-mno-sse -mno-mmx -mno-x87; see
// freestanding-port/Makefile), so clang passes/returns `double` in INTEGER registers —
// the same register file Rust's soft-float UEFI/none ABI uses for f64. The C
// side (math_bridge.c) hands us `double` and we take `f64` directly.
//
// This holds only while flite is soft-float: built with hardware SSE, clang
// would pass doubles in XMM0 while we read integer registers, corrupting every
// argument. Keep flite soft-float.

#[no_mangle]
pub extern "C" fn rust_ceil(x: f64) -> f64 {
    libm::ceil(x)
}
#[no_mangle]
pub extern "C" fn rust_exp(x: f64) -> f64 {
    libm::exp(x)
}
#[no_mangle]
pub extern "C" fn rust_fabs(x: f64) -> f64 {
    libm::fabs(x)
}
#[no_mangle]
pub extern "C" fn rust_fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}
#[no_mangle]
pub extern "C" fn rust_log(x: f64) -> f64 {
    libm::log(x)
}
#[no_mangle]
pub extern "C" fn rust_pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
#[no_mangle]
pub extern "C" fn rust_sin(x: f64) -> f64 {
    libm::sin(x)
}
#[no_mangle]
pub extern "C" fn rust_sqrt(x: f64) -> f64 {
    libm::sqrt(x)
}

// ---------------------------------------------------------------------------
// stdlib
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn abort() -> ! {
    panic!("flite C code called abort()");
}

#[no_mangle]
pub extern "C" fn exit(code: c_int) -> ! {
    panic!("flite C code called exit({code})");
}

#[no_mangle]
pub extern "C" fn abs(n: c_int) -> c_int {
    n.wrapping_abs()
}

#[no_mangle]
pub unsafe extern "C" fn atoi(s: *const c_char) -> c_int {
    // Clamp to int range (C `atoi` overflow is UB; glibc saturates). Matters for
    // long numeric input from text normalization.
    atoi_inner(s).clamp(c_int::MIN as i64, c_int::MAX as i64) as c_int
}

// Hand-rolled rather than str::parse: atoi scans a digit prefix (not the whole
// string) and saturates on overflow; parse() rejects both.
unsafe fn atoi_inner(s: *const c_char) -> i64 {
    if s.is_null() {
        return 0;
    }
    let mut i = 0usize;
    // skip leading whitespace
    while is_space(*s.add(i)) {
        i += 1;
    }
    let mut neg = false;
    match *s.add(i) as u8 {
        b'+' => i += 1,
        b'-' => {
            neg = true;
            i += 1;
        }
        _ => {}
    }
    let mut val: i64 = 0;
    loop {
        let c = *s.add(i) as u8;
        if c.is_ascii_digit() {
            // Saturate at i64 bounds; atoi() then clamps to int range.
            val = val.saturating_mul(10).saturating_add((c - b'0') as i64);
            i += 1;
        } else {
            break;
        }
    }
    if neg {
        -val
    } else {
        val
    }
}

/// `atof`: parse a C string to f64. The C side (math_bridge.c `cstm_atof`)
/// receives this `f64` directly (see the math bridge note above).
#[no_mangle]
pub unsafe extern "C" fn rust_atof(s: *const c_char) -> f64 {
    if s.is_null() {
        return 0.0;
    }
    // Collect a parseable prefix into a small stack buffer, then use Rust's parser.
    let mut buf = [0u8; 64];
    let mut n = 0usize;
    let mut i = 0usize;
    while is_space(*s.add(i)) {
        i += 1;
    }
    // sign
    let c0 = *s.add(i) as u8;
    if (c0 == b'+' || c0 == b'-') && n < buf.len() {
        buf[n] = c0;
        n += 1;
        i += 1;
    }
    let mut seen_dot = false;
    let mut seen_exp = false;
    loop {
        if n >= buf.len() - 1 {
            break;
        }
        let c = *s.add(i) as u8;
        if c.is_ascii_digit() {
            buf[n] = c;
            n += 1;
            i += 1;
        } else if c == b'.' && !seen_dot && !seen_exp {
            seen_dot = true;
            buf[n] = c;
            n += 1;
            i += 1;
        } else if (c == b'e' || c == b'E') && !seen_exp {
            seen_exp = true;
            buf[n] = c;
            n += 1;
            i += 1;
            // optional sign after exponent
            let cs = *s.add(i) as u8;
            if (cs == b'+' || cs == b'-') && n < buf.len() - 1 {
                buf[n] = cs;
                n += 1;
                i += 1;
            }
        } else {
            break;
        }
    }
    let v: f64 = match core::str::from_utf8(&buf[..n]) {
        Ok(st) => st.parse::<f64>().unwrap_or(0.0),
        Err(_) => 0.0,
    };
    v
}

#[inline]
fn is_space(c: c_char) -> bool {
    matches!(c as u8, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

// Simple LCG. flite only uses rand() for some non-critical jitter paths.
static mut RAND_STATE: u64 = 0x2545F4914F6CDD1D;

#[no_mangle]
pub extern "C" fn rand() -> c_int {
    unsafe {
        RAND_STATE = RAND_STATE
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((RAND_STATE >> 33) as u32 & 0x7fff_ffff) as c_int
    }
}

// ---------------------------------------------------------------------------
// stdio / file / posix  --  STUBS
// ---------------------------------------------------------------------------
//
// The compiled-in const voice never loads files, so these all fail/no-op.
// If any is actually invoked during synthesis it is a bug to surface later.

// stdin/stdout/stderr are only ever passed to our stubbed fprintf family.
#[no_mangle]
pub static mut stdin: *mut c_void = core::ptr::null_mut();
#[no_mangle]
pub static mut stdout: *mut c_void = core::ptr::null_mut();
#[no_mangle]
pub static mut stderr: *mut c_void = core::ptr::null_mut();

#[no_mangle]
pub extern "C" fn fopen(_path: *const c_char, _mode: *const c_char) -> *mut c_void {
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn fclose(_stream: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn fread(
    _ptr: *mut c_void,
    _size: usize,
    _nmemb: usize,
    _stream: *mut c_void,
) -> usize {
    0
}

#[no_mangle]
pub extern "C" fn fwrite(
    _ptr: *const c_void,
    _size: usize,
    _nmemb: usize,
    _stream: *mut c_void,
) -> usize {
    0
}

#[no_mangle]
pub extern "C" fn fseek(_stream: *mut c_void, _offset: c_long, _whence: c_int) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn ftell(_stream: *mut c_void) -> c_long {
    -1
}

#[no_mangle]
pub extern "C" fn fgetc(_stream: *mut c_void) -> c_int {
    -1 // EOF
}

#[no_mangle]
pub extern "C" fn fileno(_stream: *mut c_void) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn fstat(_fd: c_int, _buf: *mut c_void) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn open(_path: *const c_char, _flags: c_int) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn read(_fd: c_int, _buf: *mut c_void, _count: usize) -> isize {
    0
}

#[no_mangle]
pub extern "C" fn close(_fd: c_int) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn mmap(
    _addr: *mut c_void,
    _length: usize,
    _prot: c_int,
    _flags: c_int,
    _fd: c_int,
    _offset: c_long,
) -> *mut c_void {
    // MAP_FAILED
    (-1isize) as *mut c_void
}

#[no_mangle]
pub extern "C" fn munmap(_addr: *mut c_void, _length: usize) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn getpagesize() -> c_int {
    4096
}

#[no_mangle]
pub unsafe extern "C" fn perror(s: *const c_char) {
    // In std mode we route to stderr (UEFI console). In no_std mode flite's
    // perror calls are silent — embedders (e.g. the GRUB module) usually
    // don't have a meaningful stderr anyway, and the diagnostic strings flite
    // emits here ("Cannot open ...") are about file paths that won't exist
    // in pre-boot regardless.
    #[cfg(feature = "std")]
    {
        if !s.is_null() && *s != 0 {
            let len = strlen(s);
            let slice = core::slice::from_raw_parts(s as *const u8, len);
            if let Ok(st) = core::str::from_utf8(slice) {
                std::eprintln!("{st}");
            }
        }
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = s;
    }
}
