#ifndef _UEFI_SHIM_ASSERT_H_
#define _UEFI_SHIM_ASSERT_H_

/* Disable assertions in UEFI freestanding environment */
#define assert(x) ((void)0)

#endif /* _UEFI_SHIM_ASSERT_H_ */
