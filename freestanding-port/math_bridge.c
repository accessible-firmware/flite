/* Floating-point bridge: routes flite's libm calls to the Rust shim.
 *
 * flite is built with soft-float (-mno-sse -mno-mmx -mno-x87; see
 * freestanding-port/Makefile), so clang passes/returns `double` in INTEGER registers —
 * the same register file Rust's soft-float UEFI/none ABI uses for f64. The C
 * and Rust ABIs therefore agree, and we forward `double` straight through to
 * the Rust shim (which implements the math via libm). flite's <math.h>/
 * <stdlib.h> #define sqrt/exp/.../atof to these cstm_* entry points.
 *
 * This relies on flite being soft-float: with hardware SSE, clang would pass
 * doubles in XMM0 while Rust reads them from integer registers, corrupting
 * every argument. Keep flite soft-float (see freestanding-port/Makefile). */

extern double rust_ceil(double);
extern double rust_exp(double);
extern double rust_fabs(double);
extern double rust_fmod(double, double);
extern double rust_log(double);
extern double rust_pow(double, double);
extern double rust_sin(double);
extern double rust_sqrt(double);
extern double rust_atof(const char *);

double cstm_ceil(double x)          { return rust_ceil(x); }
double cstm_exp(double x)           { return rust_exp(x); }
double cstm_fabs(double x)          { return rust_fabs(x); }
double cstm_fmod(double x, double y){ return rust_fmod(x, y); }
double cstm_log(double x)           { return rust_log(x); }
double cstm_pow(double x, double y) { return rust_pow(x, y); }
double cstm_sin(double x)           { return rust_sin(x); }
double cstm_sqrt(double x)          { return rust_sqrt(x); }
double cstm_atof(const char *s)     { return rust_atof(s); }
