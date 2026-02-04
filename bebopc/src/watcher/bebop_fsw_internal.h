#ifndef BEBOP_FSW_INTERNAL_H
#define BEBOP_FSW_INTERNAL_H

#ifndef _WIN32
#include <pthread.h>
#endif
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "../bebopc_glob.h"
#include "bebop_fsw.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  int platform_wd;
  bebop_fsw_action_t action;
  bebop_fsw_flags_t flags;
  char* path;
  char* old_path;
  uint64_t inode;
} _fsw_raw_event_t;

void _fsw_raw_event_free(_fsw_raw_event_t* event);

typedef struct {
  bebop_fsw_watch_id_t id;
  int platform_wd;
  char* path;
  size_t path_len;
  bool recursive;
  bool ignore_hidden;
  bool follow_symlinks;
  bebopc_glob_t* glob;
} _fsw_watch_t;

typedef struct _fsw_platform _fsw_platform_t;

_fsw_platform_t* _fsw_platform_create(void);

void _fsw_platform_destroy(_fsw_platform_t* platform);

int _fsw_platform_add_watch(_fsw_platform_t* platform,
                            const char* path,
                            bool recursive);

void _fsw_platform_remove_watch(_fsw_platform_t* platform, int wd);

int _fsw_platform_poll(_fsw_platform_t* platform,
                       int timeout_ms,
                       _fsw_raw_event_t* events,
                       size_t max_events);

bool _fsw_matches_filter(bebopc_glob_t* glob, const char* rel_path);

#ifdef __cplusplus
}
#endif

#endif
