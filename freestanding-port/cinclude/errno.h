#ifndef _UEFI_SHIM_ERRNO_H_
#define _UEFI_SHIM_ERRNO_H_

/* Minimal errno for UEFI freestanding environment */
extern int errno;

#define EPERM    1
#define ENOENT   2
#define ESRCH    3
#define EINTR    4
#define EIO      5
#define ENXIO    6
#define ENOEXEC  8
#define EBADF    9
#define ENOMEM   12
#define EACCES   13
#define EFAULT   14
#define EBUSY    16
#define EEXIST   17
#define ENODEV   19
#define ENOTDIR  20
#define EISDIR   21
#define EINVAL   22
#define ENFILE   23
#define EMFILE   24
#define ENOSPC   28
#define EROFS    30
#define EPIPE    32
#define ERANGE   34
#define ENOSYS   38

#endif /* _UEFI_SHIM_ERRNO_H_ */
