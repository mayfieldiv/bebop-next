#ifndef BEBOPC_GLOB_H
#define BEBOPC_GLOB_H

#include <stdbool.h>
#include <stddef.h>

#include "bebopc.h"
#include "bebopc_error.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct bebopc_glob bebopc_glob_t;

typedef struct bebopc_glob_config {
  bool case_sensitive;
  bool preserve_order;
  bool follow_symlinks;
} bebopc_glob_config_t;

typedef struct bebopc_glob_match {
  char* path;
  char* stem;
} bebopc_glob_match_t;

typedef struct bebopc_glob_result {
  bebopc_glob_match_t* matches;
  size_t count;
  size_t capacity;
} bebopc_glob_result_t;

#define BEBOPC_GLOB_CONFIG_DEFAULT \
  ((bebopc_glob_config_t) {false, false, false})

bebopc_glob_t* bebopc_glob_new(bebopc_glob_config_t config);

void bebopc_glob_free(bebopc_glob_t* glob);

bebopc_error_code_t bebopc_glob_include(bebopc_glob_t* glob,
                                        const char* pattern);

bebopc_error_code_t bebopc_glob_exclude(bebopc_glob_t* glob,
                                        const char* pattern);

bebopc_error_code_t bebopc_glob_include_many(bebopc_glob_t* glob,
                                             const char** patterns,
                                             size_t count);

bebopc_error_code_t bebopc_glob_exclude_many(bebopc_glob_t* glob,
                                             const char** patterns,
                                             size_t count);

bebopc_glob_result_t* bebopc_glob_execute(bebopc_glob_t* glob,
                                          bebopc_ctx_t* ctx,
                                          const char* directory);

char** bebopc_glob_paths(bebopc_glob_t* glob,
                         bebopc_ctx_t* ctx,
                         const char* directory,
                         size_t* out_count);

void bebopc_glob_paths_free(char** paths, size_t count);

void bebopc_glob_result_free(bebopc_glob_result_t* result);

bool bebopc_glob_match(bebopc_glob_t* glob,
                       const char* pattern,
                       const char* path);

bool bebopc_glob_match_segment(bebopc_glob_t* glob,
                               const char* pattern,
                               const char* name);

bool bebopc_glob_filter(bebopc_glob_t* glob, const char* path);

bool bebopc_glob_is_pattern(const char* str);

#ifdef __cplusplus
}
#endif

#endif
