#include "bebop_fsw_internal.h"

#if defined(BEBOP_FSW_LINUX)

#include <assert.h>

struct _fsw_platform {
  int dummy;
};

_fsw_platform_t* _fsw_platform_create(void)
{
  assert(0 && "Linux file watcher not implemented");
  return NULL;
}

void _fsw_platform_destroy(_fsw_platform_t* p)
{
  (void)p;
  assert(0 && "Linux file watcher not implemented");
}

int _fsw_platform_add_watch(_fsw_platform_t* p, const char* path, bool recursive)
{
  (void)p;
  (void)path;
  (void)recursive;
  assert(0 && "Linux file watcher not implemented");
  return BEBOP_FSW_ERR_SYSTEM;
}

void _fsw_platform_remove_watch(_fsw_platform_t* p, int wd)
{
  (void)p;
  (void)wd;
  assert(0 && "Linux file watcher not implemented");
}

int _fsw_platform_poll(
    _fsw_platform_t* p, int timeout_ms, _fsw_raw_event_t* events, size_t max_events
)
{
  (void)p;
  (void)timeout_ms;
  (void)events;
  (void)max_events;
  assert(0 && "Linux file watcher not implemented");
  return BEBOP_FSW_ERR_SYSTEM;
}

#endif
