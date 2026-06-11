#ifndef _UEFI_SHIM_WCHAR_H_
#define _UEFI_SHIM_WCHAR_H_

#include <stddef.h>

/* wchar_t is defined in stddef.h by clang's freestanding headers */

size_t wcslen(const wchar_t *s);
wchar_t *wcscpy(wchar_t *dest, const wchar_t *src);
wchar_t *wcscat(wchar_t *dest, const wchar_t *src);
int     wcscmp(const wchar_t *s1, const wchar_t *s2);
size_t  mbstowcs(wchar_t *dest, const char *src, size_t n);
size_t  wcstombs(char *dest, const wchar_t *src, size_t n);

#endif /* _UEFI_SHIM_WCHAR_H_ */
