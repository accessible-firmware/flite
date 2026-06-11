#ifndef _UEFI_SHIM_UNISTD_H_
#define _UEFI_SHIM_UNISTD_H_

#include <stddef.h>

typedef long ssize_t;
typedef int  pid_t;

/* POSIX I/O (declared for compilation; stub implementations from Rust shim) */
ssize_t read(int fd, void *buf, size_t count);
ssize_t write(int fd, const void *buf, size_t count);
int     close(int fd);
int     getpagesize(void);

#endif /* _UEFI_SHIM_UNISTD_H_ */
