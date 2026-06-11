#ifndef _UEFI_SHIM_FCNTL_H_
#define _UEFI_SHIM_FCNTL_H_

#define O_RDONLY  0
#define O_WRONLY  1
#define O_RDWR    2
#define O_CREAT   0100
#define O_TRUNC   01000
#define O_APPEND  02000

int open(const char *path, int flags, ...);

#endif /* _UEFI_SHIM_FCNTL_H_ */
