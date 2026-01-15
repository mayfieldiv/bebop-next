#ifndef BEBOP_FSW_H
#define BEBOP_FSW_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32) || defined(_WIN64)
#define BEBOP_FSW_WINDOWS 1
#elif defined(__APPLE__)
#define BEBOP_FSW_MACOS 1
#elif defined(__FreeBSD__) || defined(__NetBSD__) || defined(__OpenBSD__) \
    || defined(__DragonFly__)
#define BEBOP_FSW_BSD 1
#elif defined(__linux__)
#define BEBOP_FSW_LINUX 1
#else
#error "Unsupported platform"
#endif

typedef struct bebop_fsw bebop_fsw_t;

typedef int32_t bebop_fsw_watch_id_t;

typedef enum {
  BEBOP_FSW_ACTION_ADD = 1,
  BEBOP_FSW_ACTION_DELETE = 2,
  BEBOP_FSW_ACTION_MODIFY = 3,
  BEBOP_FSW_ACTION_RENAME = 4,
} bebop_fsw_action_t;

typedef enum {
  BEBOP_FSW_FLAG_NONE = 0,
  BEBOP_FSW_FLAG_DIR = 1 << 0,
} bebop_fsw_flags_t;

typedef struct {
  bebop_fsw_watch_id_t watch_id;
  bebop_fsw_action_t action;
  bebop_fsw_flags_t flags;
  const char* dir;
  const char* name;
  const char* old_name;
} bebop_fsw_event_t;

typedef void (*bebop_fsw_callback_t)(const bebop_fsw_event_t* event,
                                     void* userdata);

typedef enum {
  BEBOP_FSW_OK = 0,
  BEBOP_FSW_ERR_NOMEM = -1,
  BEBOP_FSW_ERR_NOT_FOUND = -2,
  BEBOP_FSW_ERR_NOT_DIR = -3,
  BEBOP_FSW_ERR_ACCESS = -4,
  BEBOP_FSW_ERR_LIMIT = -5,
  BEBOP_FSW_ERR_EXISTS = -6,
  BEBOP_FSW_ERR_INVALID = -7,
  BEBOP_FSW_ERR_SYSTEM = -8,
  BEBOP_FSW_ERR_OVERFLOW = -9,
  BEBOP_FSW_ERR_CLOSED = -10,
} bebop_fsw_result_t;

typedef struct {
  const char** include;
  size_t include_count;
  const char** exclude;
  size_t exclude_count;
} bebop_fsw_filter_t;

typedef struct {
  bool recursive;
  bool ignore_hidden;
  bool follow_symlinks;
  const bebop_fsw_filter_t* filter;
} bebop_fsw_options_t;

bebop_fsw_t* bebop_fsw_create(bebop_fsw_callback_t callback, void* userdata);

void bebop_fsw_destroy(bebop_fsw_t* fsw);

extern const bebop_fsw_options_t BEBOP_FSW_OPTIONS_DEFAULT;

bebop_fsw_result_t bebop_fsw_add_watch(bebop_fsw_t* fsw,
                                       const char* path,
                                       const bebop_fsw_options_t* options,
                                       bebop_fsw_watch_id_t* out_id);

bebop_fsw_result_t bebop_fsw_remove_watch(bebop_fsw_t* fsw,
                                          bebop_fsw_watch_id_t id);

bebop_fsw_result_t bebop_fsw_remove_watch_path(bebop_fsw_t* fsw,
                                               const char* path);

int bebop_fsw_poll(bebop_fsw_t* fsw, int timeout_ms);

const char* bebop_fsw_strerror(bebop_fsw_result_t err);

const char* bebop_fsw_action_name(bebop_fsw_action_t action);

#ifdef __cplusplus
}
#endif

#endif
