#ifndef BEBOPC_UTILS_H
#define BEBOPC_UTILS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) || defined(_WIN64)
#define BEBOPC_WINDOWS 1
#define BEBOPC_PATH_SEP '\\'
#define BEBOPC_PATH_SEP_STR "\\"
#else
#define BEBOPC_POSIX 1
#define BEBOPC_PATH_SEP '/'
#define BEBOPC_PATH_SEP_STR "/"
#endif

#if defined(__APPLE__) && defined(__MACH__)
#define BEBOPC_MACOS 1
#elif defined(__linux__)
#define BEBOPC_LINUX 1
#elif defined(__FreeBSD__) || defined(__NetBSD__) || defined(__OpenBSD__)
#define BEBOPC_BSD 1
#endif

#ifdef BEBOPC_WINDOWS
#include <tchar.h>
#define bebopc_char_t TCHAR
#define BEBOPC_STRING(s) _TEXT(s)
#define bebopc_strlen _tcslen
#define bebopc_strcpy _tcscpy
#define bebopc_strcat _tcscat
#define bebopc_strcmp _tcscmp
#define bebopc_strchr _tcschr
#define bebopc_strrchr _tcsrchr
#define bebopc_strncmp _tcsncmp
#else
#define bebopc_char_t char
#define BEBOPC_STRING(s) s
#define bebopc_strlen strlen
#define bebopc_strcpy strcpy
#define bebopc_strcat strcat
#define bebopc_strcmp strcmp
#define bebopc_strchr strchr
#define bebopc_strrchr strrchr
#define bebopc_strncmp strncmp
#endif

#define BEBOPC_IS_ASCII(c) (((unsigned char)(c)) < 0x80)

#define BEBOPC_IS_DIGIT(c) ((c) >= '0' && (c) <= '9')

#define BEBOPC_IS_HEX(c) \
  (BEBOPC_IS_DIGIT(c) || ((c) >= 'A' && (c) <= 'F') \
   || ((c) >= 'a' && (c) <= 'f'))

#define BEBOPC_HEX_VALUE(c) \
  (((c) >= '0' && (c) <= '9')       ? (c) - '0' \
       : ((c) >= 'A' && (c) <= 'F') ? (c) - 'A' + 10 \
       : ((c) >= 'a' && (c) <= 'f') ? (c) - 'a' + 10 \
                                    : -1)

#define BEBOPC_IS_ALPHA(c) \
  (((c) >= 'A' && (c) <= 'Z') || ((c) >= 'a' && (c) <= 'z'))

#define BEBOPC_IS_ALNUM(c) (BEBOPC_IS_ALPHA(c) || BEBOPC_IS_DIGIT(c))

#define BEBOPC_IS_WHITE(c) ((c) == ' ' || (c) == '\t')

#define BEBOPC_IS_BREAK(c) ((c) == '\n' || (c) == '\r')

#define BEBOPC_IS_BLANK(c) (BEBOPC_IS_WHITE(c) || BEBOPC_IS_BREAK(c))

#define BEBOPC_IS_BLANKZ(c) (BEBOPC_IS_BLANK(c) || (c) == '\0')

#define BEBOPC_TOLOWER(c) (((c) >= 'A' && (c) <= 'Z') ? (c) + 32 : (c))
#define BEBOPC_TOUPPER(c) (((c) >= 'a' && (c) <= 'z') ? (c) - 32 : (c))

#define BEBOPC_CHAR_IEQ(a, b) (BEBOPC_TOLOWER(a) == BEBOPC_TOLOWER(b))

char* bebopc_strdup(const char* s);
char* bebopc_strndup(const char* s, size_t n);
size_t bebopc_strncpy(char* dest, const char* src, size_t dest_size);
bool bebopc_streq(const char* s1, const char* s2);
bool bebopc_strcaseeq(const char* s1, const char* s2);

static inline int bebopc_memicmp(const char* a, const char* b, size_t n)
{
  for (size_t i = 0; i < n; i++) {
    int d = BEBOPC_TOLOWER((unsigned char)a[i])
        - BEBOPC_TOLOWER((unsigned char)b[i]);
    if (d != 0) {
      return d;
    }
  }
  return 0;
}

void* bebopc_memdup(const void* src, size_t size);
void bebopc_files_free(const char** files, uint32_t count);

bool bebopc_random_bytes(void* buf, size_t size);
bool bebopc_random_uint64(uint64_t* result);
bool bebopc_random_uint32(uint32_t* result);

uint64_t bebopc_monotonic_ns(void);
uint64_t bebopc_monotonic_us(void);
uint64_t bebopc_monotonic_ms(void);

uint64_t bebopc_timestamp_ns(void);
uint64_t bebopc_timestamp_us(void);
uint64_t bebopc_timestamp_ms(void);

uint64_t bebopc_elapsed_ns(uint64_t start_ns, uint64_t end_ns);
uint64_t bebopc_elapsed_us(uint64_t start_ns, uint64_t end_ns);
uint64_t bebopc_elapsed_ms(uint64_t start_ns, uint64_t end_ns);

bool bebopc_format_timestamp(uint64_t timestamp_ms,
                             char* buffer,
                             size_t buffer_size);

void bebopc_console_init(void);
bool bebopc_color_supported(void);
char* bebopc_exe_path(void);

#ifdef __cplusplus
}
#endif

#endif
