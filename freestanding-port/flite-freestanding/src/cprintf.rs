//! A minimal but real C `printf`-family format engine.
//!
//! Supports the specifiers flite actually uses: `%d %i %u %ld %lu %f %g %e
//! %s %c %x %X %o %p %%`, plus width, precision, and the `-`, `0`, `+`, ` `
//! flags. Unsupported specifiers fall back to emitting the literal directive
//! text so output is never silently corrupted.
//!
//! - `snprintf`/`vsprintf` write into a caller buffer.
//! - `fprintf`/`vfprintf`/`printf`/`vprintf` write to the UEFI console via the
//!   std stdout (which UEFI std maps to the console protocol).
//!
//! On this nightly the c-variadic API is: a `mut args: ...` parameter is a
//! `core::ffi::VaList`, and individual arguments are read with the unsafe
//! method `args.next_arg::<T>()`. `VaArgSafe` requires sized types, so we read
//! `i32`/`i64`/`u32`/`u64`/`f64`/pointers explicitly (C default-promotes
//! `float`->`double` and small ints->`int`).

use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void, VaList};
#[cfg(feature = "std")]
use std::io::Write;

/// Where formatted bytes go.
enum Sink<'a> {
    /// Bounded buffer (snprintf/vsprintf): buffer, capacity, count written.
    Buf {
        buf: *mut u8,
        cap: usize,
        written: usize,
    },
    /// std stdout (UEFI console).
    Console { scratch: &'a mut Vec<u8> },
}

impl<'a> Sink<'a> {
    fn push(&mut self, byte: u8) {
        match self {
            Sink::Buf { buf, cap, written } => {
                // snprintf: store only if room for this byte plus the NUL.
                if *cap > 0 && *written + 1 < *cap {
                    unsafe {
                        *buf.add(*written) = byte;
                    }
                }
                *written += 1;
            }
            Sink::Console { scratch } => scratch.push(byte),
        }
    }

    fn push_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push(b);
        }
    }

    /// Returns the number of characters that *would* have been written
    /// (C `snprintf`/`printf` return semantics).
    fn finish(self) -> c_int {
        match self {
            Sink::Buf { buf, cap, written } => {
                if cap > 0 {
                    let idx = core::cmp::min(written, cap - 1);
                    unsafe {
                        *buf.add(idx) = 0;
                    }
                }
                written as c_int
            }
            Sink::Console { scratch } => {
                let n = scratch.len();
                // Under std, the UEFI console is reachable via std::io::stdout.
                // Under no_std the printf-family is silent — embedders that
                // care about flite's diagnostic output can capture it through
                // their own callback by patching the consumer (we expose
                // Sink::Buf for non-streaming use).
                #[cfg(feature = "std")]
                {
                    let mut out = std::io::stdout();
                    let _ = out.write_all(scratch);
                    let _ = out.flush();
                }
                n as c_int
            }
        }
    }
}

struct Flags {
    left: bool,
    zero: bool,
    plus: bool,
    space: bool,
    alt: bool,
}

fn write_padded(sink: &mut Sink, body: &[u8], width: usize, left: bool, zero: bool, prefix: &[u8]) {
    let total = body.len() + prefix.len();
    let pad = width.saturating_sub(total);
    if left {
        sink.push_bytes(prefix);
        sink.push_bytes(body);
        for _ in 0..pad {
            sink.push(b' ');
        }
    } else if zero {
        sink.push_bytes(prefix);
        for _ in 0..pad {
            sink.push(b'0');
        }
        sink.push_bytes(body);
    } else {
        for _ in 0..pad {
            sink.push(b' ');
        }
        sink.push_bytes(prefix);
        sink.push_bytes(body);
    }
}

fn emit_int(sink: &mut Sink, digits: &[u8], negative: bool, flags: &Flags, width: usize) {
    let mut prefix: Vec<u8> = Vec::new();
    if negative {
        prefix.push(b'-');
    } else if flags.plus {
        prefix.push(b'+');
    } else if flags.space {
        prefix.push(b' ');
    }
    write_padded(
        sink,
        digits,
        width,
        flags.left,
        flags.zero && !flags.left,
        &prefix,
    );
}

unsafe fn cstr_bytes<'b>(p: *const c_char) -> &'b [u8] {
    if p.is_null() {
        return b"(null)";
    }
    let mut len = 0usize;
    while *p.add(len) != 0 {
        len += 1;
    }
    core::slice::from_raw_parts(p as *const u8, len)
}

fn u64_to_digits(mut v: u64, base: u64, upper: bool) -> Vec<u8> {
    if v == 0 {
        return alloc::vec![b'0'];
    }
    let lut: &[u8] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut tmp = Vec::new();
    while v > 0 {
        tmp.push(lut[(v % base) as usize]);
        v /= base;
    }
    tmp.reverse();
    tmp
}

fn fmt_double(val: f64, spec: u8, precision: Option<usize>, flags: &Flags) -> Vec<u8> {
    use core::fmt::Write as _;
    let prec = precision.unwrap_or(6);
    let neg = val.is_sign_negative() && !val.is_nan();
    let absval = libm::fabs(val);

    let mut body = String::new();
    match spec {
        b'f' | b'F' => {
            let _ = write!(body, "{:.*}", prec, absval);
        }
        b'e' | b'E' => {
            let _ = write!(body, "{:.*e}", prec, absval);
            if spec == b'E' {
                body = body.to_uppercase();
            }
        }
        b'g' | b'G' => {
            let p = if prec == 0 { 1 } else { prec };
            body = format_g(absval, p);
            if spec == b'G' {
                body = body.to_uppercase();
            }
        }
        _ => {
            let _ = write!(body, "{:.*}", prec, absval);
        }
    }

    let mut s = String::new();
    if neg {
        s.push('-');
    } else if flags.plus {
        s.push('+');
    } else if flags.space {
        s.push(' ');
    }
    s.push_str(&body);
    s.into_bytes()
}

fn format_g(val: f64, prec: usize) -> String {
    use core::fmt::Write as _;
    let mut s = String::new();
    if val == 0.0 {
        return String::from("0");
    }
    let exp = libm::floor(libm::log10(val)) as i32;
    if exp < -4 || exp >= prec as i32 {
        let _ = write!(s, "{:.*e}", prec.saturating_sub(1), val);
    } else {
        let dec = (prec as i32 - 1 - exp).max(0) as usize;
        let _ = write!(s, "{:.*}", dec, val);
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
    }
    s
}

/// Core engine. Walks the format string and pulls arguments from `args`.
///
/// # Safety
/// `fmt` must be a valid NUL-terminated string and `args` must contain
/// arguments matching the directives in `fmt`.
unsafe fn vformat(mut sink: Sink, fmt: *const c_char, mut args: VaList) -> c_int {
    if fmt.is_null() {
        return sink.finish();
    }
    let mut i = 0usize;
    loop {
        let c = *fmt.add(i) as u8;
        if c == 0 {
            break;
        }
        if c != b'%' {
            sink.push(c);
            i += 1;
            continue;
        }
        let directive_start = i;
        i += 1; // consume '%'

        let mut flags = Flags {
            left: false,
            zero: false,
            plus: false,
            space: false,
            alt: false,
        };
        loop {
            match *fmt.add(i) as u8 {
                b'-' => flags.left = true,
                b'0' => flags.zero = true,
                b'+' => flags.plus = true,
                b' ' => flags.space = true,
                b'#' => flags.alt = true,
                _ => break,
            }
            i += 1;
        }

        // width
        let mut width: usize = 0;
        if *fmt.add(i) as u8 == b'*' {
            let w = args.next_arg::<i32>();
            if w < 0 {
                flags.left = true;
                width = (-(w as i64)) as usize;
            } else {
                width = w as usize;
            }
            i += 1;
        } else {
            while (*fmt.add(i) as u8).is_ascii_digit() {
                width = width * 10 + (*fmt.add(i) as u8 - b'0') as usize;
                i += 1;
            }
        }

        // precision
        let mut precision: Option<usize> = None;
        if *fmt.add(i) as u8 == b'.' {
            i += 1;
            if *fmt.add(i) as u8 == b'*' {
                let p = args.next_arg::<i32>();
                precision = Some(if p < 0 { 0 } else { p as usize });
                i += 1;
            } else {
                let mut p = 0usize;
                while (*fmt.add(i) as u8).is_ascii_digit() {
                    p = p * 10 + (*fmt.add(i) as u8 - b'0') as usize;
                    i += 1;
                }
                precision = Some(p);
            }
        }

        // length modifiers. On x86_64-unknown-uefi (IL32P64/Win64) `long` is
        // 32-bit, so a single `l` reads a 32-bit arg; `ll` is 64-bit, and
        // `z`/`j`/`t`/`q` (size_t/intmax_t/ptrdiff_t) are 64-bit. Reading the
        // wrong width would pick up caller-garbage in the high bits and
        // misalign subsequent args, so count the `l`s rather than collapse them.
        let mut l_count = 0u8;
        let mut width64 = false;
        loop {
            match *fmt.add(i) as u8 {
                b'l' => {
                    l_count += 1;
                    i += 1;
                }
                b'z' | b'j' | b't' | b'q' => {
                    width64 = true;
                    i += 1;
                }
                b'h' | b'L' => {
                    i += 1;
                }
                _ => break,
            }
        }
        let long = width64 || l_count >= 2;

        let conv = *fmt.add(i) as u8;
        i += 1;

        match conv {
            b'%' => sink.push(b'%'),
            b'c' => {
                let ch = args.next_arg::<i32>() as u8;
                write_padded(&mut sink, &[ch], width, flags.left, false, b"");
            }
            b's' => {
                let p = args.next_arg::<*const c_char>();
                let mut bytes = cstr_bytes(p);
                if let Some(prec) = precision {
                    if prec < bytes.len() {
                        bytes = &bytes[..prec];
                    }
                }
                write_padded(&mut sink, bytes, width, flags.left, false, b"");
            }
            b'd' | b'i' => {
                let v: i64 = if long {
                    args.next_arg::<i64>()
                } else {
                    args.next_arg::<i32>() as i64
                };
                let neg = v < 0;
                let mag = if neg {
                    (v as i128).unsigned_abs() as u64
                } else {
                    v as u64
                };
                let digits = u64_to_digits(mag, 10, false);
                emit_int(&mut sink, &digits, neg, &flags, width);
            }
            b'u' => {
                let v: u64 = if long {
                    args.next_arg::<u64>()
                } else {
                    args.next_arg::<u32>() as u64
                };
                let digits = u64_to_digits(v, 10, false);
                emit_int(&mut sink, &digits, false, &flags, width);
            }
            b'x' | b'X' => {
                let v: u64 = if long {
                    args.next_arg::<u64>()
                } else {
                    args.next_arg::<u32>() as u64
                };
                let digits = u64_to_digits(v, 16, conv == b'X');
                let prefix: &[u8] = if flags.alt && v != 0 {
                    if conv == b'X' {
                        b"0X"
                    } else {
                        b"0x"
                    }
                } else {
                    b""
                };
                write_padded(
                    &mut sink,
                    &digits,
                    width,
                    flags.left,
                    flags.zero && !flags.left,
                    prefix,
                );
            }
            b'o' => {
                let v: u64 = if long {
                    args.next_arg::<u64>()
                } else {
                    args.next_arg::<u32>() as u64
                };
                let digits = u64_to_digits(v, 8, false);
                emit_int(&mut sink, &digits, false, &flags, width);
            }
            b'p' => {
                let v = args.next_arg::<*const c_void>() as usize as u64;
                let digits = u64_to_digits(v, 16, false);
                write_padded(&mut sink, &digits, width, flags.left, false, b"0x");
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                let v = args.next_arg::<f64>();
                let body = fmt_double(v, conv, precision, &flags);
                write_padded(
                    &mut sink,
                    &body,
                    width,
                    flags.left,
                    flags.zero && !flags.left,
                    b"",
                );
            }
            0 => {
                // malformed: trailing '%'
                sink.push(b'%');
                break;
            }
            _ => {
                // Unsupported specifier: emit the raw directive verbatim.
                let mut j = directive_start;
                while j < i {
                    sink.push(*fmt.add(j) as u8);
                    j += 1;
                }
            }
        }
    }
    sink.finish()
}

// ---------------------------------------------------------------------------
// Public C entry points
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn vsprintf(buf: *mut c_char, fmt: *const c_char, args: VaList) -> c_int {
    let sink = Sink::Buf {
        buf: buf as *mut u8,
        cap: usize::MAX,
        written: 0,
    };
    vformat(sink, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vfprintf(_stream: *mut c_void, fmt: *const c_char, args: VaList) -> c_int {
    // Our stdin/stdout/stderr stubs are all null, so the stream argument cannot
    // discriminate; route all diagnostic output to the console.
    let mut scratch = Vec::new();
    let sink = Sink::Console {
        scratch: &mut scratch,
    };
    vformat(sink, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn snprintf(
    buf: *mut c_char,
    n: usize,
    fmt: *const c_char,
    args: ...
) -> c_int {
    let sink = Sink::Buf {
        buf: buf as *mut u8,
        cap: n,
        written: 0,
    };
    vformat(sink, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn fprintf(_stream: *mut c_void, fmt: *const c_char, args: ...) -> c_int {
    let mut scratch = Vec::new();
    let sink = Sink::Console {
        scratch: &mut scratch,
    };
    vformat(sink, fmt, args)
}
