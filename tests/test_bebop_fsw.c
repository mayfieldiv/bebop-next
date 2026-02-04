#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebopc_dir.h"
#include "bebopc_utils.h"
#include "unity.h"
#include "watcher/bebop_fsw.h"

#ifdef _WIN32
#include <windows.h>
#define sleep_ms(ms) Sleep(ms)
#else
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#define sleep_ms(ms) usleep((ms) * 1000)
#endif

static char* test_dir = NULL;
static bebop_fsw_event_t last_event;
static int event_count = 0;

static char* create_test_dir(void)
{
  char template[] = "/tmp/fsw_test_XXXXXX";
#ifdef _WIN32
  char* dir = _mktemp(template);
  if (dir) {
    _mkdir(dir);
  }
  return dir ? bebopc_strdup(dir) : NULL;
#else
  char* dir = mkdtemp(template);
  return dir ? bebopc_strdup(dir) : NULL;
#endif
}

static void remove_test_dir_recursive(const char* path)
{
  if (!path) {
    return;
  }

#ifdef _WIN32
  WIN32_FIND_DATAA fd;
  char pattern[BEBOPC_PATH_MAX];
  snprintf(pattern, sizeof(pattern), "%s\\*", path);

  HANDLE h = FindFirstFileA(pattern, &fd);
  if (h == INVALID_HANDLE_VALUE) {
    return;
  }

  do {
    if (strcmp(fd.cFileName, ".") == 0 || strcmp(fd.cFileName, "..") == 0) {
      continue;
    }

    char* child = bebopc_path_join(path, fd.cFileName);
    if (!child) {
      continue;
    }

    if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
      remove_test_dir_recursive(child);
    } else {
      DeleteFileA(child);
    }
    free(child);
  } while (FindNextFileA(h, &fd));

  FindClose(h);
  RemoveDirectoryA(path);
#else
  DIR* dir = opendir(path);
  if (!dir) {
    return;
  }

  struct dirent* entry;
  while ((entry = readdir(dir)) != NULL) {
    if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
      continue;
    }

    char* child = bebopc_path_join(path, entry->d_name);
    if (!child) {
      continue;
    }

    struct stat st;
    if (stat(child, &st) == 0) {
      if (S_ISDIR(st.st_mode)) {
        remove_test_dir_recursive(child);
      } else {
        unlink(child);
      }
    }
    free(child);
  }

  closedir(dir);
  rmdir(path);
#endif
}

static void remove_test_dir(const char* path)
{
  remove_test_dir_recursive(path);
}

static void create_file(const char* dir, const char* name)
{
  char* path = bebopc_path_join(dir, name);
  if (path) {
    FILE* f = fopen(path, "w");
    if (f) {
      fprintf(f, "test content\n");
      fclose(f);
    }
    free(path);
  }
}

static void modify_file(const char* dir, const char* name)
{
  char* path = bebopc_path_join(dir, name);
  if (path) {
    FILE* f = fopen(path, "a");
    if (f) {
      fprintf(f, "more content\n");
      fclose(f);
    }
    free(path);
  }
}

static void delete_file(const char* dir, const char* name)
{
  char* path = bebopc_path_join(dir, name);
  if (path) {
    remove(path);
    free(path);
  }
}

static void rename_file(const char* dir, const char* old_name, const char* new_name)
{
  char* old_path = bebopc_path_join(dir, old_name);
  char* new_path = bebopc_path_join(dir, new_name);
  if (old_path && new_path) {
    rename(old_path, new_path);
  }
  free(old_path);
  free(new_path);
}

static void create_subdir(const char* dir, const char* name)
{
  char* path = bebopc_path_join(dir, name);
  if (path) {
#ifdef _WIN32
    _mkdir(path);
#else
    mkdir(path, 0755);
#endif
    free(path);
  }
}

static void test_callback(const bebop_fsw_event_t* event, void* userdata)
{
  (void)userdata;

  last_event.watch_id = event->watch_id;
  last_event.action = event->action;
  last_event.flags = event->flags;

  event_count++;

  printf(
      "  Event: %s %s/%s\n",
      bebop_fsw_action_name(event->action),
      event->dir ? event->dir : "",
      event->name ? event->name : ""
  );
}

static void reset_events(void)
{
  memset(&last_event, 0, sizeof(last_event));
  event_count = 0;
}

void setUp(void)
{
  test_dir = create_test_dir();
  TEST_ASSERT_NOT_NULL_MESSAGE(test_dir, "Failed to create test directory");
  reset_events();
}

void tearDown(void)
{
  if (test_dir) {
    remove_test_dir(test_dir);
    free(test_dir);
    test_dir = NULL;
  }
}

void test_fsw_create_destroy(void);
void test_fsw_create_null_callback(void);

void test_fsw_create_destroy(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);
  bebop_fsw_destroy(fsw);
}

void test_fsw_create_null_callback(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(NULL, NULL);
  TEST_ASSERT_NULL(fsw);
}

void test_fsw_add_watch(void);
void test_fsw_add_watch_invalid_path(void);
void test_fsw_add_watch_not_dir(void);
void test_fsw_add_watch_duplicate(void);
void test_fsw_remove_watch(void);
void test_fsw_remove_watch_by_path(void);

void test_fsw_add_watch(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_watch_id_t id;
  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, &id);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);
  TEST_ASSERT_GREATER_THAN(0, id);

  bebop_fsw_destroy(fsw);
}

void test_fsw_add_watch_invalid_path(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, "/nonexistent/path/12345", NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_ERR_NOT_FOUND, result);

  bebop_fsw_destroy(fsw);
}

void test_fsw_add_watch_not_dir(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  create_file(test_dir, "not_a_dir.txt");
  char* file_path = bebopc_path_join(test_dir, "not_a_dir.txt");

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, file_path, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_ERR_NOT_DIR, result);

  free(file_path);
  bebop_fsw_destroy(fsw);
}

void test_fsw_add_watch_duplicate(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_ERR_EXISTS, result);

  bebop_fsw_destroy(fsw);
}

void test_fsw_remove_watch(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_watch_id_t id;
  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, &id);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  result = bebop_fsw_remove_watch(fsw, id);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  result = bebop_fsw_add_watch(fsw, test_dir, NULL, &id);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  bebop_fsw_destroy(fsw);
}

void test_fsw_remove_watch_by_path(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  result = bebop_fsw_remove_watch_path(fsw, test_dir);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  bebop_fsw_destroy(fsw);
}

void test_fsw_poll_timeout(void);
void test_fsw_poll_nonblocking(void);

void test_fsw_poll_timeout(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  int count = bebop_fsw_poll(fsw, 50);
  TEST_ASSERT_EQUAL(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_poll_nonblocking(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  int count = bebop_fsw_poll(fsw, 0);
  TEST_ASSERT_GREATER_OR_EQUAL(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_event_create(void);
void test_fsw_event_modify(void);
void test_fsw_event_delete(void);
void test_fsw_event_rename(void);

void test_fsw_event_create(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);

  printf("Creating file...\n");
  create_file(test_dir, "new_file.txt");

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events\n", count);

  TEST_ASSERT_GREATER_THAN(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_event_modify(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  create_file(test_dir, "existing.txt");

  sleep_ms(100);

  bebop_fsw_poll(fsw, 100);
  reset_events();

  printf("Modifying file...\n");
  modify_file(test_dir, "existing.txt");

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events\n", count);

  TEST_ASSERT_GREATER_THAN(0, count);

  TEST_ASSERT_TRUE(
      last_event.action == BEBOP_FSW_ACTION_MODIFY || last_event.action == BEBOP_FSW_ACTION_ADD
  );

  bebop_fsw_destroy(fsw);
}

void test_fsw_event_delete(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  create_file(test_dir, "to_delete.txt");

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  printf("Deleting file...\n");
  delete_file(test_dir, "to_delete.txt");

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events\n", count);

  TEST_ASSERT_GREATER_THAN(0, count);
  TEST_ASSERT_EQUAL(BEBOP_FSW_ACTION_DELETE, last_event.action);

  bebop_fsw_destroy(fsw);
}

void test_fsw_event_rename(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  create_file(test_dir, "old_name.txt");

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, NULL, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  printf("Renaming file...\n");
  rename_file(test_dir, "old_name.txt", "new_name.txt");

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events\n", count);

  TEST_ASSERT_GREATER_THAN(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_recursive(void);
void test_fsw_non_recursive(void);
void test_fsw_ignore_hidden(void);

void test_fsw_recursive(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  create_subdir(test_dir, "subdir");

  bebop_fsw_options_t opts = BEBOP_FSW_OPTIONS_DEFAULT;
  opts.recursive = true;

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, &opts, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  char* subdir = bebopc_path_join(test_dir, "subdir");
  printf("Creating file in subdirectory...\n");
  create_file(subdir, "nested.txt");
  free(subdir);

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events (recursive=true)\n", count);

  TEST_ASSERT_GREATER_THAN(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_non_recursive(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_options_t opts = BEBOP_FSW_OPTIONS_DEFAULT;
  opts.recursive = false;

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, &opts, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);

  bebop_fsw_poll(fsw, 0);
  reset_events();

  create_subdir(test_dir, "subdir2");
  sleep_ms(100);

  bebop_fsw_poll(fsw, 100);
  reset_events();

  char* subdir = bebopc_path_join(test_dir, "subdir2");
  printf("Creating file in subdirectory (non-recursive)...\n");
  create_file(subdir, "nested.txt");
  free(subdir);

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events (recursive=false)\n", count);

  TEST_ASSERT_EQUAL(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_ignore_hidden(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  bebop_fsw_options_t opts = BEBOP_FSW_OPTIONS_DEFAULT;
  opts.ignore_hidden = true;

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, &opts, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  printf("Creating hidden file...\n");
  create_file(test_dir, ".hidden");

  sleep_ms(200);
  int count = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events (ignore_hidden=true)\n", count);

  TEST_ASSERT_EQUAL(0, count);

  bebop_fsw_destroy(fsw);
}

void test_fsw_filter_include(void);
void test_fsw_filter_exclude(void);

void test_fsw_filter_include(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  const char* include[] = {"*.txt"};
  bebop_fsw_filter_t filter = {
      .include = include, .include_count = 1, .exclude = NULL, .exclude_count = 0
  };

  bebop_fsw_options_t opts = BEBOP_FSW_OPTIONS_DEFAULT;
  opts.filter = &filter;

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, &opts, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  printf("Creating test.txt...\n");
  create_file(test_dir, "test.txt");

  sleep_ms(200);
  int count1 = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events for .txt file\n", count1);

  reset_events();

  printf("Creating test.json...\n");
  create_file(test_dir, "test.json");

  sleep_ms(200);
  int count2 = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events for .json file\n", count2);

  TEST_ASSERT_GREATER_THAN(0, count1);
  TEST_ASSERT_EQUAL(0, count2);

  bebop_fsw_destroy(fsw);
}

void test_fsw_filter_exclude(void)
{
  bebop_fsw_t* fsw = bebop_fsw_create(test_callback, NULL);
  TEST_ASSERT_NOT_NULL(fsw);

  const char* include[] = {"*"};
  const char* exclude[] = {"*.log"};
  bebop_fsw_filter_t filter = {
      .include = include, .include_count = 1, .exclude = exclude, .exclude_count = 1
  };

  bebop_fsw_options_t opts = BEBOP_FSW_OPTIONS_DEFAULT;
  opts.filter = &filter;

  bebop_fsw_result_t result = bebop_fsw_add_watch(fsw, test_dir, &opts, NULL);
  TEST_ASSERT_EQUAL(BEBOP_FSW_OK, result);

  sleep_ms(100);
  reset_events();

  printf("Creating test.log...\n");
  create_file(test_dir, "test.log");

  sleep_ms(200);
  int count1 = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events for .log file\n", count1);

  reset_events();

  printf("Creating test2.txt...\n");
  create_file(test_dir, "test2.txt");

  sleep_ms(200);
  int count2 = bebop_fsw_poll(fsw, 500);
  printf("Poll returned %d events for .txt file\n", count2);

  TEST_ASSERT_EQUAL(0, count1);
  TEST_ASSERT_GREATER_THAN(0, count2);

  bebop_fsw_destroy(fsw);
}

void test_fsw_strerror(void);
void test_fsw_action_name(void);

void test_fsw_strerror(void)
{
  TEST_ASSERT_EQUAL_STRING("Success", bebop_fsw_strerror(BEBOP_FSW_OK));
  TEST_ASSERT_EQUAL_STRING("Out of memory", bebop_fsw_strerror(BEBOP_FSW_ERR_NOMEM));
  TEST_ASSERT_EQUAL_STRING("Path not found", bebop_fsw_strerror(BEBOP_FSW_ERR_NOT_FOUND));
  TEST_ASSERT_EQUAL_STRING("Path is not a directory", bebop_fsw_strerror(BEBOP_FSW_ERR_NOT_DIR));
  TEST_ASSERT_EQUAL_STRING("Permission denied", bebop_fsw_strerror(BEBOP_FSW_ERR_ACCESS));
  TEST_ASSERT_EQUAL_STRING("Invalid argument", bebop_fsw_strerror(BEBOP_FSW_ERR_INVALID));
}

void test_fsw_action_name(void)
{
  TEST_ASSERT_EQUAL_STRING("add", bebop_fsw_action_name(BEBOP_FSW_ACTION_ADD));
  TEST_ASSERT_EQUAL_STRING("delete", bebop_fsw_action_name(BEBOP_FSW_ACTION_DELETE));
  TEST_ASSERT_EQUAL_STRING("modify", bebop_fsw_action_name(BEBOP_FSW_ACTION_MODIFY));
  TEST_ASSERT_EQUAL_STRING("rename", bebop_fsw_action_name(BEBOP_FSW_ACTION_RENAME));
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_fsw_create_destroy);
  RUN_TEST(test_fsw_create_null_callback);

  RUN_TEST(test_fsw_add_watch);
  RUN_TEST(test_fsw_add_watch_invalid_path);
  RUN_TEST(test_fsw_add_watch_not_dir);
  RUN_TEST(test_fsw_add_watch_duplicate);
  RUN_TEST(test_fsw_remove_watch);
  RUN_TEST(test_fsw_remove_watch_by_path);

  RUN_TEST(test_fsw_poll_timeout);
  RUN_TEST(test_fsw_poll_nonblocking);

  RUN_TEST(test_fsw_event_create);
  RUN_TEST(test_fsw_event_modify);
  RUN_TEST(test_fsw_event_delete);
  RUN_TEST(test_fsw_event_rename);

  RUN_TEST(test_fsw_recursive);
  RUN_TEST(test_fsw_non_recursive);
  RUN_TEST(test_fsw_ignore_hidden);

  RUN_TEST(test_fsw_filter_include);
  RUN_TEST(test_fsw_filter_exclude);

  RUN_TEST(test_fsw_strerror);
  RUN_TEST(test_fsw_action_name);

  return UNITY_END();
}
