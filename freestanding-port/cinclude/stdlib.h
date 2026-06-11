#ifndef _UEFI_SHIM_STDLIB_H_
#define _UEFI_SHIM_STDLIB_H_

#include <stddef.h>

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
/* Must match the range of the shim's rand() (0 .. 2^31-1). flite's mixed-
 * excitation code does `rand() > RAND_MAX/2.0`, so a too-small RAND_MAX makes
 * that test always true and silences the synthesis. 

- See: `filte/src/cg/cst_mlsa.c:plus_or_minus_one()` */
#define RAND_MAX     2147483647

#ifndef NULL
#define NULL ((void *)0)
#endif

/* Memory management */
void  *malloc(size_t size);
void  *calloc(size_t nmemb, size_t size);
void  *realloc(void *ptr, size_t size);
void   free(void *ptr);

/* Program control */
void   abort(void);
void   exit(int status);

/* String/number conversion */
int    atoi(const char *s);
double cstm_atof(const char *s);
#define atof cstm_atof
double strtod(const char *s, char **endptr);
long   strtol(const char *s, char **endptr, int base);

/* Utilities:
For those coming from Rust, qsort is more naturally expressed with the following syntax:

qsort<T, F>(base: *T, len: usize, size: sizeof(T), comp: F)
where:
	F: Fn(*T, *T) -> Compare
*/
void   qsort(void *base, size_t nmemb, size_t size,
             int (*compar)(const void *, const void *));
int    rand(void);
void   srand(unsigned int seed);
char  *getenv(const char *name);
int    abs(int j);

#endif /* _UEFI_SHIM_STDLIB_H_ */
