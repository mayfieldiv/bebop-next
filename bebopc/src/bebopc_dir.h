#ifndef BEBOPC_DIR_H
#define BEBOPC_DIR_H

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable : 4996)
#endif

#include <errno.h>
#include <stdlib.h>

#include "bebopc.h"
#include "bebopc_error.h"
#include "bebopc_utils.h"

#ifdef __cplusplus
extern "C" {
#endif

#if ((defined _UNICODE) && !(defined UNICODE))
#define UNICODE
#endif
#if ((defined UNICODE) && !(defined _UNICODE))
#define _UNICODE
#endif

#ifdef BEBOPC_WINDOWS
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <direct.h>
#include <tchar.h>
#include <windows.h>
#else
#include <dirent.h>
#include <libgen.h>
#include <limits.h>
#include <sys/stat.h>
#include <unistd.h>
#endif

#ifdef BEBOPC_WINDOWS
#define BEBOPC_PATH_MAX MAX_PATH
#define BEBOPC_PATH_EXTRA 2
#define BEBOPC_DRIVE_MAX 3
#else
#ifdef PATH_MAX
#define BEBOPC_PATH_MAX PATH_MAX
#else
#define BEBOPC_PATH_MAX 4096
#endif
#define BEBOPC_PATH_EXTRA 0
#endif

#define BEBOPC_FILENAME_MAX 256

#if defined(i386) || defined(__i386__) || defined(__i386) || defined(_M_IX86)
#ifdef _MSC_VER
#define BEBOPC_CDECL __cdecl
#else
#define BEBOPC_CDECL __attribute__((cdecl))
#endif
#else
#define BEBOPC_CDECL
#endif

#ifndef BEBOPC_WINDOWS
#if (defined __MINGW32__) && (defined _UNICODE)
#define BEBOPC_DIR_HANDLE _WDIR
#define bebopc_dirent _wdirent
#define bebopc_opendir _wopendir
#define bebopc_readdir _wreaddir
#define bebopc_closedir _wclosedir
#else
#define BEBOPC_DIR_HANDLE DIR
#define bebopc_dirent dirent
#define bebopc_opendir opendir
#define bebopc_readdir readdir
#define bebopc_closedir closedir
#endif
#endif

typedef struct bebopc_file {
  bebopc_char_t path[BEBOPC_PATH_MAX];
  bebopc_char_t name[BEBOPC_FILENAME_MAX];
  bebopc_char_t* extension;
  bool is_dir;
  bool is_reg;
#ifndef BEBOPC_WINDOWS
#ifdef __MINGW32__
  struct _stat _s;
#else
  struct stat _s;
#endif
#endif
} bebopc_file_t;

typedef struct bebopc_dir {
  bebopc_ctx_t* ctx;
  bebopc_char_t path[BEBOPC_PATH_MAX];
  bool has_next;
  size_t n_files;

  bebopc_file_t* _files;
#ifdef BEBOPC_WINDOWS
  HANDLE _h;
  WIN32_FIND_DATA _f;
#else
  BEBOPC_DIR_HANDLE* _d;
  struct bebopc_dirent* _e;
#endif
} bebopc_dir_t;

bebopc_error_code_t bebopc_dir_open(bebopc_dir_t* dir,
                                    bebopc_ctx_t* ctx,
                                    const bebopc_char_t* path);
bebopc_error_code_t bebopc_dir_open_sorted(bebopc_dir_t* dir,
                                           bebopc_ctx_t* ctx,
                                           const bebopc_char_t* path);
void bebopc_dir_close(bebopc_dir_t* dir);
bebopc_error_code_t bebopc_dir_next(bebopc_dir_t* dir);
bebopc_error_code_t bebopc_dir_readfile(const bebopc_dir_t* dir,
                                        bebopc_file_t* file);
bebopc_error_code_t bebopc_dir_readfile_n(const bebopc_dir_t* dir,
                                          bebopc_file_t* file,
                                          size_t i);
bebopc_error_code_t bebopc_dir_open_subdir_n(bebopc_dir_t* dir, size_t i);

bebopc_error_code_t bebopc_file_open(bebopc_file_t* file,
                                     bebopc_ctx_t* ctx,
                                     const bebopc_char_t* path);

void _bebopc_get_ext(bebopc_file_t* file);
int BEBOPC_CDECL _bebopc_file_cmp(const void* a, const void* b);

bool bebopc_path_is_absolute(const bebopc_char_t* path);
bool bebopc_file_exists(const bebopc_char_t* path);
bool bebopc_file_is_dir(const bebopc_char_t* path);
bool bebopc_file_is_file(const bebopc_char_t* path);

char* bebopc_path_join(const char* dir, const char* file);
char* bebopc_path_dirname(const char* path);
const char* bebopc_path_basename(const char* path);
const char* bebopc_path_extension(const char* path);
char* bebopc_path_normalize(const char* path);
char* bebopc_path_realpath(const char* path);
char* bebopc_path_absolute(bebopc_ctx_t* ctx, const char* path);
char* bebopc_path_relative(const char* base, const char* path);

bool bebopc_path_is_hidden(const char* path);
bool bebopc_path_has_separator(const char* path);

char* bebopc_file_read(bebopc_ctx_t* ctx, const char* path, size_t* out_size);
bool bebopc_file_write(bebopc_ctx_t* ctx,
                       const char* path,
                       const char* data,
                       size_t size);
bool bebopc_mkdir(bebopc_ctx_t* ctx, const char* path);

char* bebopc_getcwd(void);

#ifdef __cplusplus
}
#endif

#ifdef _MSC_VER
#pragma warning(pop)
#endif

#endif
