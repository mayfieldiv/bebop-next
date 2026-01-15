#include "bebop_fsw.h"

#include <stdlib.h>
#include <string.h>

#include "../bebopc_dir.h"
#include "../bebopc_glob.h"
#include "../bebopc_utils.h"
#include "bebop_fsw_internal.h"

#define FSW_INITIAL_WATCHES 4
#define FSW_MAX_EVENTS_PER_POLL 64

struct bebop_fsw {
  bebop_fsw_callback_t callback;
  void* userdata;

  _fsw_platform_t* platform;

  _fsw_watch_t* watches;
  size_t watch_count;
  size_t watch_capacity;
  bebop_fsw_watch_id_t next_id;

  bool closed;
};

const bebop_fsw_options_t BEBOP_FSW_OPTIONS_DEFAULT = {
    .recursive = false, .ignore_hidden = false, .follow_symlinks = false, .filter = NULL
};

void _fsw_raw_event_free(_fsw_raw_event_t* event)
{
  if (!event) {
    return;
  }
  free(event->path);
  free(event->old_path);
  event->path = NULL;
  event->old_path = NULL;
}

bool _fsw_matches_filter(bebopc_glob_t* glob, const char* rel_path)
{
  if (!glob) {
    return true;
  }
  if (!rel_path || !*rel_path) {
    return false;
  }

  return bebopc_glob_filter(glob, rel_path);
}

static _fsw_watch_t* _fsw_find_watch_by_path(bebop_fsw_t* fsw, const char* path)
{
  for (size_t i = 0; i < fsw->watch_count; i++) {
    if (strcmp(fsw->watches[i].path, path) == 0) {
      return &fsw->watches[i];
    }
  }
  return NULL;
}

static _fsw_watch_t* _fsw_find_watch_by_platform_wd(bebop_fsw_t* fsw, int platform_wd)
{
  for (size_t i = 0; i < fsw->watch_count; i++) {
    if (fsw->watches[i].platform_wd == platform_wd) {
      return &fsw->watches[i];
    }
  }
  return NULL;
}

static void _fsw_watch_cleanup(_fsw_watch_t* watch)
{
  if (!watch) {
    return;
  }
  free(watch->path);
  watch->path = NULL;
  if (watch->glob) {
    bebopc_glob_free(watch->glob);
    watch->glob = NULL;
  }
}

static bebopc_glob_t* _fsw_create_glob_filter(
    const bebop_fsw_filter_t* filter, bool follow_symlinks
)
{
  if (!filter) {
    return NULL;
  }

  if ((!filter->include || filter->include_count == 0)
      && (!filter->exclude || filter->exclude_count == 0))
  {
    return NULL;
  }

  bebopc_glob_t* glob = bebopc_glob_new((bebopc_glob_config_t) {
      .case_sensitive = false, .preserve_order = false, .follow_symlinks = follow_symlinks
  });
  if (!glob) {
    return NULL;
  }

  if (filter->include && filter->include_count > 0) {
    for (size_t i = 0; i < filter->include_count; i++) {
      if (bebopc_glob_include(glob, filter->include[i]) != BEBOPC_OK) {
        bebopc_glob_free(glob);
        return NULL;
      }
    }
  } else {
    if (bebopc_glob_include(glob, "**/*") != BEBOPC_OK) {
      bebopc_glob_free(glob);
      return NULL;
    }
  }

  if (filter->exclude && filter->exclude_count > 0) {
    for (size_t i = 0; i < filter->exclude_count; i++) {
      if (bebopc_glob_exclude(glob, filter->exclude[i]) != BEBOPC_OK) {
        bebopc_glob_free(glob);
        return NULL;
      }
    }
  }

  return glob;
}

static bool _fsw_should_report(
    _fsw_watch_t* watch, const char* full_path, bebop_fsw_action_t action
)
{
  if (!watch || !full_path) {
    return false;
  }

  char* rel_path = bebopc_path_relative(watch->path, full_path);
  if (!rel_path) {
    return false;
  }

  if (rel_path[0] == '\0') {
    free(rel_path);

    return action == BEBOP_FSW_ACTION_DELETE || action == BEBOP_FSW_ACTION_RENAME;
  }

  if (!watch->recursive && bebopc_path_has_separator(rel_path)) {
    free(rel_path);
    return false;
  }

  if (watch->ignore_hidden && bebopc_path_is_hidden(full_path)) {
    free(rel_path);
    return false;
  }

  if (watch->glob && !_fsw_matches_filter(watch->glob, rel_path)) {
    free(rel_path);
    return false;
  }

  free(rel_path);
  return true;
}

bebop_fsw_t* bebop_fsw_create(bebop_fsw_callback_t callback, void* userdata)
{
  if (!callback) {
    return NULL;
  }

  bebop_fsw_t* fsw = calloc(1, sizeof(bebop_fsw_t));
  if (!fsw) {
    return NULL;
  }

  fsw->callback = callback;
  fsw->userdata = userdata;
  fsw->next_id = 1;

  fsw->platform = _fsw_platform_create();
  if (!fsw->platform) {
    free(fsw);
    return NULL;
  }

  return fsw;
}

void bebop_fsw_destroy(bebop_fsw_t* fsw)
{
  if (!fsw) {
    return;
  }

  fsw->closed = true;

  for (size_t i = 0; i < fsw->watch_count; i++) {
    _fsw_platform_remove_watch(fsw->platform, fsw->watches[i].platform_wd);
    _fsw_watch_cleanup(&fsw->watches[i]);
  }
  free(fsw->watches);

  _fsw_platform_destroy(fsw->platform);

  free(fsw);
}

bebop_fsw_result_t bebop_fsw_add_watch(
    bebop_fsw_t* fsw,
    const char* path,
    const bebop_fsw_options_t* options,
    bebop_fsw_watch_id_t* out_id
)
{
  if (out_id) {
    *out_id = 0;
  }
  if (!fsw || !path) {
    return BEBOP_FSW_ERR_INVALID;
  }
  if (fsw->closed) {
    return BEBOP_FSW_ERR_CLOSED;
  }

  const bebop_fsw_options_t* opts = options ? options : &BEBOP_FSW_OPTIONS_DEFAULT;

  char* norm_path = bebopc_path_normalize(path);
  if (!norm_path) {
    return BEBOP_FSW_ERR_NOMEM;
  }

  char* real_path = bebopc_path_realpath(norm_path);
  free(norm_path);
  if (!real_path) {
    return BEBOP_FSW_ERR_NOT_FOUND;
  }
  norm_path = real_path;

  if (!bebopc_file_exists(norm_path)) {
    free(norm_path);
    return BEBOP_FSW_ERR_NOT_FOUND;
  }
  if (!bebopc_file_is_dir(norm_path)) {
    free(norm_path);
    return BEBOP_FSW_ERR_NOT_DIR;
  }

  if (_fsw_find_watch_by_path(fsw, norm_path)) {
    free(norm_path);
    return BEBOP_FSW_ERR_EXISTS;
  }

  if (fsw->watch_count >= fsw->watch_capacity) {
    size_t new_cap = fsw->watch_capacity == 0 ? FSW_INITIAL_WATCHES : fsw->watch_capacity * 2;
    _fsw_watch_t* new_watches = realloc(fsw->watches, new_cap * sizeof(_fsw_watch_t));
    if (!new_watches) {
      free(norm_path);
      return BEBOP_FSW_ERR_NOMEM;
    }
    fsw->watches = new_watches;
    fsw->watch_capacity = new_cap;
  }

  bebopc_glob_t* glob = _fsw_create_glob_filter(opts->filter, opts->follow_symlinks);

  int platform_wd = _fsw_platform_add_watch(fsw->platform, norm_path, opts->recursive);
  if (platform_wd < 0) {
    free(norm_path);
    if (glob) {
      bebopc_glob_free(glob);
    }
    return (bebop_fsw_result_t)platform_wd;
  }

  _fsw_watch_t* watch = &fsw->watches[fsw->watch_count];
  watch->id = fsw->next_id++;
  watch->platform_wd = platform_wd;
  watch->path = norm_path;
  watch->path_len = strlen(norm_path);
  watch->recursive = opts->recursive;
  watch->ignore_hidden = opts->ignore_hidden;
  watch->follow_symlinks = opts->follow_symlinks;
  watch->glob = glob;

  fsw->watch_count++;

  if (out_id) {
    *out_id = watch->id;
  }
  return BEBOP_FSW_OK;
}

bebop_fsw_result_t bebop_fsw_remove_watch(bebop_fsw_t* fsw, bebop_fsw_watch_id_t id)
{
  if (!fsw) {
    return BEBOP_FSW_ERR_INVALID;
  }
  if (fsw->closed) {
    return BEBOP_FSW_ERR_CLOSED;
  }

  for (size_t i = 0; i < fsw->watch_count; i++) {
    if (fsw->watches[i].id == id) {
      _fsw_platform_remove_watch(fsw->platform, fsw->watches[i].platform_wd);
      _fsw_watch_cleanup(&fsw->watches[i]);

      for (size_t j = i + 1; j < fsw->watch_count; j++) {
        fsw->watches[j - 1] = fsw->watches[j];
      }
      fsw->watch_count--;
      return BEBOP_FSW_OK;
    }
  }

  return BEBOP_FSW_ERR_NOT_FOUND;
}

bebop_fsw_result_t bebop_fsw_remove_watch_path(bebop_fsw_t* fsw, const char* path)
{
  if (!fsw || !path) {
    return BEBOP_FSW_ERR_INVALID;
  }
  if (fsw->closed) {
    return BEBOP_FSW_ERR_CLOSED;
  }

  char* real_path = bebopc_path_realpath(path);
  if (!real_path) {
    return BEBOP_FSW_ERR_NOT_FOUND;
  }

  _fsw_watch_t* watch = _fsw_find_watch_by_path(fsw, real_path);
  free(real_path);

  if (!watch) {
    return BEBOP_FSW_ERR_NOT_FOUND;
  }

  return bebop_fsw_remove_watch(fsw, watch->id);
}

int bebop_fsw_poll(bebop_fsw_t* fsw, int timeout_ms)
{
  if (!fsw) {
    return BEBOP_FSW_ERR_INVALID;
  }
  if (fsw->closed) {
    return BEBOP_FSW_ERR_CLOSED;
  }

  _fsw_raw_event_t raw_events[FSW_MAX_EVENTS_PER_POLL];
  int raw_count =
      _fsw_platform_poll(fsw->platform, timeout_ms, raw_events, FSW_MAX_EVENTS_PER_POLL);

  if (raw_count < 0) {
    return raw_count;
  }

  int reported = 0;
  for (int i = 0; i < raw_count; i++) {
    _fsw_raw_event_t* raw = &raw_events[i];

    _fsw_watch_t* watch = _fsw_find_watch_by_platform_wd(fsw, raw->platform_wd);
    if (!watch) {
      _fsw_raw_event_free(raw);
      continue;
    }

    if (!_fsw_should_report(watch, raw->path, raw->action)) {
      _fsw_raw_event_free(raw);
      continue;
    }

    char* rel_path = bebopc_path_relative(watch->path, raw->path);
    const char* name = rel_path ? bebopc_path_basename(rel_path) : bebopc_path_basename(raw->path);

    char* dir = NULL;
    if (rel_path && name > rel_path) {
      size_t dir_len = (size_t)(name - rel_path - 1);
      if (dir_len > 0) {
        dir = bebopc_strndup(rel_path, dir_len);
      }
    }

    char* full_dir = NULL;
    if (dir && dir[0] != '\0') {
      full_dir = bebopc_path_join(watch->path, dir);
    } else {
      full_dir = bebopc_strdup(watch->path);
    }

    const char* old_name = NULL;
    if (raw->old_path && raw->action == BEBOP_FSW_ACTION_RENAME) {
      old_name = bebopc_path_basename(raw->old_path);
    }

    bebop_fsw_event_t event = {
        .watch_id = watch->id,
        .action = raw->action,
        .flags = raw->flags,
        .dir = full_dir,
        .name = name,
        .old_name = old_name
    };

    fsw->callback(&event, fsw->userdata);
    reported++;

    free(full_dir);
    free(dir);
    free(rel_path);
    _fsw_raw_event_free(raw);
  }

  return reported;
}

const char* bebop_fsw_strerror(bebop_fsw_result_t err)
{
  switch (err) {
    case BEBOP_FSW_OK:
      return "Success";
    case BEBOP_FSW_ERR_NOMEM:
      return "Out of memory";
    case BEBOP_FSW_ERR_NOT_FOUND:
      return "Path not found";
    case BEBOP_FSW_ERR_NOT_DIR:
      return "Path is not a directory";
    case BEBOP_FSW_ERR_ACCESS:
      return "Permission denied";
    case BEBOP_FSW_ERR_LIMIT:
      return "System watch limit reached";
    case BEBOP_FSW_ERR_EXISTS:
      return "Watch already exists";
    case BEBOP_FSW_ERR_INVALID:
      return "Invalid argument";
    case BEBOP_FSW_ERR_SYSTEM:
      return "System error";
    case BEBOP_FSW_ERR_OVERFLOW:
      return "Event queue overflow";
    case BEBOP_FSW_ERR_CLOSED:
      return "Watcher closed";
    default:
      return "Unknown error";
  }
}

const char* bebop_fsw_action_name(bebop_fsw_action_t action)
{
  switch (action) {
    case BEBOP_FSW_ACTION_ADD:
      return "add";
    case BEBOP_FSW_ACTION_DELETE:
      return "delete";
    case BEBOP_FSW_ACTION_MODIFY:
      return "modify";
    case BEBOP_FSW_ACTION_RENAME:
      return "rename";
    default:
      return "unknown";
  }
}
