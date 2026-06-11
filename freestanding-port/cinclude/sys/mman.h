#ifndef _UEFI_SHIM_SYS_MMAN_H_
#define _UEFI_SHIM_SYS_MMAN_H_

#include <sys/types.h>

#define PROT_READ    0x1
#define PROT_WRITE   0x2
#define PROT_EXEC    0x4
#define MAP_SHARED   0x01
#define MAP_PRIVATE  0x02
#define MAP_FAILED   ((void *)-1)
#define MAP_NOSYNCFILE 0

typedef unsigned char caddr_t;  /* used in mmap return cast in cst_mmap_posix.c */

void  *mmap(void *addr, size_t length, int prot, int flags, int fd, off_t offset);
int    munmap(void *addr, size_t length);

#endif /* _UEFI_SHIM_SYS_MMAN_H_ */
