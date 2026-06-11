#ifndef CST_UEFI_SETJMP_SHIM
#define CST_UEFI_SETJMP_SHIM
/* Freestanding <setjmp.h> shim for the flite UEFI / ELF cross-build.
 *
 * flite's error handler (src/utils/cst_error.c) unconditionally declares
 *   jmp_buf *cst_errjmp = 0;
 * and, in the non-DIE_ON_ERROR configuration, uses setjmp()/longjmp() for
 * error recovery. This port builds with -DDIE_ON_ERROR, so cst_error()
 * expands to abort() and nothing ever calls setjmp() to arm `cst_errjmp`
 * (it stays NULL for the life of the process). longjmp() is therefore
 * never reached at runtime.
 *
 * Consequently we only need the `jmp_buf` TYPE to exist so the translation
 * units that mention it will compile. The two functions are declared (so
 * any stray reference is a visible link error rather than silent UB) but
 * never defined, because they are never called. Previously the build
 * happened to pick up a host <setjmp.h> from the system include path; that
 * is fragile, so we provide an explicit one here. */
typedef long jmp_buf[8];
int  setjmp(jmp_buf env);
void longjmp(jmp_buf env, int val);
#endif /* CST_UEFI_SETJMP_SHIM */
