#ifndef BEBOP_TEST_COMMON_H
#define BEBOP_TEST_COMMON_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#ifdef _WIN32
#ifndef S_ISREG
#define S_ISREG(m) (((m) & _S_IFMT) == _S_IFREG)
#endif
#endif

#include "bebop.h"

static void* test_alloc_fn(void* ptr, size_t old, size_t new, void* ctx)
{
  (void)old;
  (void)ctx;
  if (new == 0) {
    free(ptr);
    return NULL;
  }
  return realloc(ptr, new);
}

static inline bebop_host_allocator_t test_allocator(void)
{
  return (bebop_host_allocator_t) {.alloc = test_alloc_fn, .ctx = NULL};
}

static bebop_file_result_t test_file_reader_fn(const char* path, void* ctx)
{
  (void)ctx;
  FILE* f = fopen(path, "rb");
  if (!f) {
    return (bebop_file_result_t) {.content = NULL, .content_len = 0, .error = "File not found"};
  }
  fseek(f, 0, SEEK_END);
  long len = ftell(f);
  fseek(f, 0, SEEK_SET);
  char* content = (char*)malloc((size_t)len + 1);
  if (!content) {
    fclose(f);
    return (bebop_file_result_t) {.content = NULL, .content_len = 0, .error = "Out of memory"};
  }
  size_t read = fread(content, 1, (size_t)len, f);
  fclose(f);
  content[read] = '\0';
  return (bebop_file_result_t) {.content = content, .content_len = read, .error = NULL};
}

static bool test_file_exists_fn(const char* path, void* ctx)
{
  (void)ctx;
  struct stat st;
  return stat(path, &st) == 0 && S_ISREG(st.st_mode);
}

static inline bebop_host_t test_host(const char** include_paths, uint32_t include_path_count)
{
  bebop_host_t host = {0};
  host.allocator.alloc = test_alloc_fn;
  host.allocator.ctx = NULL;
  host.file_reader.read = test_file_reader_fn;
  host.file_reader.exists = test_file_exists_fn;
  host.file_reader.ctx = NULL;
  host.includes.paths = include_paths;
  host.includes.count = include_path_count;
  return host;
}

static inline bebop_context_t* test_context_create(void)
{
  bebop_host_t host = test_host(NULL, 0);
  return bebop_context_create(&host);
}

#endif
