#ifndef _UEFI_SHIM_STDIO_H_
#define _UEFI_SHIM_STDIO_H_

#include <stddef.h>
#include <stdarg.h>

#define EOF     (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

typedef struct _FILE FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

/* File operations */
FILE   *fopen(const char *path, const char *mode);
int     fclose(FILE *fp);
size_t  fread(void *ptr, size_t size, size_t nmemb, FILE *fp);
size_t  fwrite(const void *ptr, size_t size, size_t nmemb, FILE *fp);
int     fseek(FILE *fp, long offset, int whence);
long    ftell(FILE *fp);
int     fgetc(FILE *fp);
int     feof(FILE *fp);
int     fflush(FILE *fp);
int     fputc(int c, FILE *fp);
int     fputs(const char *s, FILE *fp);
char   *fgets(char *s, int n, FILE *fp);
void    rewind(FILE *fp);

/* For LFS -- just alias to the non-LFS versions for UEFI */
typedef long off_t;
int     fseeko(FILE *fp, off_t offset, int whence);
off_t   ftello(FILE *fp);

/* File descriptor bridge */
FILE   *fdopen(int fd, const char *mode);
int     fileno(FILE *fp);

/* Formatted output */
int     fprintf(FILE *fp, const char *fmt, ...);
int     printf(const char *fmt, ...);
int     sprintf(char *str, const char *fmt, ...);
int     snprintf(char *str, size_t size, const char *fmt, ...);
int     vsnprintf(char *str, size_t size, const char *fmt, va_list ap);
int     vfprintf(FILE *fp, const char *fmt, va_list ap);
int     vsprintf(char *str, const char *fmt, va_list ap);

/* Error reporting */
void    perror(const char *s);

#endif /* _UEFI_SHIM_STDIO_H_ */
