#ifndef _UEFI_SHIM_SYS_STAT_H_
#define _UEFI_SHIM_SYS_STAT_H_

#include <sys/types.h>

struct stat {
    dev_t     st_dev;
    ino_t     st_ino;
    mode_t    st_mode;
    nlink_t   st_nlink;
    uid_t     st_uid;
    gid_t     st_gid;
    dev_t     st_rdev;
    off_t     st_size;
    blksize_t st_blksize;
    blkcnt_t  st_blocks;
};

int fstat(int fd, struct stat *buf);
int stat(const char *path, struct stat *buf);

#endif /* _UEFI_SHIM_SYS_STAT_H_ */
