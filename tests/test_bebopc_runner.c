#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebopc_log.h"
#include "cli/bebopc_config.h"
#include "cli/bebopc_runner.h"
#include "unity.h"

bebopc_log_ctx_t* g_log_ctx;
static bebopc_log_ctx_t log_ctx;
static bebopc_ctx_t ctx;
static bebopc_config_t cfg;

static void* _test_alloc(void* ptr, size_t old, size_t new, void* udata)
{
  (void)old;
  (void)udata;
  if (new == 0) {
    free(ptr);
    return NULL;
  }
  return realloc(ptr, new);
}

static bebop_file_result_t _test_file_read(const char* path, void* udata)
{
  (void)path;
  (void)udata;
  return (bebop_file_result_t) {.error = "not implemented"};
}

static bool _test_file_exists(const char* path, void* udata)
{
  (void)path;
  (void)udata;
  return false;
}

void setUp(void);
void tearDown(void);

void setUp(void)
{
  bebopc_log_ctx_init(&log_ctx, false);
  g_log_ctx = &log_ctx;

  bebopc_ctx_init(&ctx);
  bebopc_config_init(&cfg);

  ctx.host.allocator.alloc = _test_alloc;
  ctx.host.file_reader.read = _test_file_read;
  ctx.host.file_reader.exists = _test_file_exists;

  ctx.cfg = &cfg;
}

void tearDown(void)
{
  bebopc_config_cleanup(&cfg);
  bebopc_ctx_cleanup(&ctx);
}

static bebop_descriptor_t* _make_test_desc(bebop_context_t* beb_ctx)
{
  const char* src = "struct Empty {}";
  bebop_source_t source = {.source = src, .len = strlen(src), .path = "test.bop"};
  bebop_parse_result_t* result = NULL;
  if (bebop_parse_source(beb_ctx, &source, &result) == BEBOP_FATAL) {
    return NULL;
  }
  bebop_descriptor_t* desc = NULL;
  bebop_descriptor_build(result, BEBOP_DESC_FLAG_NONE, &desc);
  return desc;
}

void test_runner_init_null_runner(void);

void test_runner_init_null_runner(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebop_descriptor_t* desc = _make_test_desc(beb_ctx);
  TEST_ASSERT_NOT_NULL(desc);

  bebopc_error_code_t err = bebopc_runner_init(NULL, &ctx, beb_ctx, desc, NULL, 0);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_INVALID_ARG, err);

  bebop_descriptor_free(desc);
  bebop_context_destroy(beb_ctx);
}

void test_runner_init_null_ctx(void);

void test_runner_init_null_ctx(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebop_descriptor_t* desc = _make_test_desc(beb_ctx);
  TEST_ASSERT_NOT_NULL(desc);

  bebopc_runner_t runner;
  bebopc_error_code_t err = bebopc_runner_init(&runner, NULL, beb_ctx, desc, NULL, 0);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_INVALID_ARG, err);

  bebop_descriptor_free(desc);
  bebop_context_destroy(beb_ctx);
}

void test_runner_init_null_beb_ctx(void);

void test_runner_init_null_beb_ctx(void)
{
  bebop_descriptor_t* desc = (bebop_descriptor_t*)0x1;

  bebopc_runner_t runner;
  bebopc_error_code_t err = bebopc_runner_init(&runner, &ctx, NULL, desc, NULL, 0);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_INVALID_ARG, err);
}

void test_runner_init_null_desc(void);

void test_runner_init_null_desc(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebopc_runner_t runner;
  bebopc_error_code_t err = bebopc_runner_init(&runner, &ctx, beb_ctx, NULL, NULL, 0);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_INVALID_ARG, err);

  bebop_context_destroy(beb_ctx);
}

void test_runner_init_files_count_mismatch(void);

void test_runner_init_files_count_mismatch(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebop_descriptor_t* desc = _make_test_desc(beb_ctx);
  TEST_ASSERT_NOT_NULL(desc);

  bebopc_runner_t runner;

  bebopc_error_code_t err = bebopc_runner_init(&runner, &ctx, beb_ctx, desc, NULL, 2);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_INVALID_ARG, err);

  bebop_descriptor_free(desc);
  bebop_context_destroy(beb_ctx);
}

void test_runner_init_cleanup(void);

void test_runner_init_cleanup(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebop_descriptor_t* desc = _make_test_desc(beb_ctx);
  TEST_ASSERT_NOT_NULL(desc);

  bebopc_runner_t runner;
  bebopc_error_code_t err = bebopc_runner_init(&runner, &ctx, beb_ctx, desc, NULL, 0);
  TEST_ASSERT_EQUAL(BEBOPC_OK, err);

  TEST_ASSERT_EQUAL_PTR(beb_ctx, runner.beb_ctx);
  TEST_ASSERT_EQUAL_PTR(&ctx, runner.ctx);
  TEST_ASSERT_EQUAL_PTR(desc, runner.desc);
  TEST_ASSERT_NULL(runner.input_files);
  TEST_ASSERT_EQUAL(0, runner.input_file_count);

  bebopc_runner_cleanup(&runner);
  TEST_ASSERT_NULL(runner.beb_ctx);
}

void test_runner_init_with_files(void);

void test_runner_init_with_files(void)
{
  bebop_context_t* beb_ctx = bebop_context_create(&ctx.host);
  TEST_ASSERT_NOT_NULL(beb_ctx);

  bebop_descriptor_t* desc = _make_test_desc(beb_ctx);
  TEST_ASSERT_NOT_NULL(desc);

  const char** files = malloc(2 * sizeof(char*));
  files[0] = strdup("file1.bop");
  files[1] = strdup("file2.bop");

  bebopc_runner_t runner;
  bebopc_error_code_t err = bebopc_runner_init(&runner, &ctx, beb_ctx, desc, files, 2);
  TEST_ASSERT_EQUAL(BEBOPC_OK, err);

  TEST_ASSERT_EQUAL_PTR(files, runner.input_files);
  TEST_ASSERT_EQUAL(2, runner.input_file_count);

  bebopc_runner_cleanup(&runner);
}

void test_find_plugin_not_found(void);

void test_find_plugin_not_found(void)
{
  char* path = bebopc_find_plugin("nonexistent-generator-xyz");
  TEST_ASSERT_NULL(path);
}

void test_find_plugin_null(void);

void test_find_plugin_null(void)
{
  TEST_ASSERT_NULL(bebopc_find_plugin(NULL));
}

void test_process_spawn_not_found(void);

void test_process_spawn_not_found(void)
{
  bebopc_process_t* p = bebopc_process_spawn("/nonexistent/binary/path");

  if (p) {
    int code = bebopc_process_wait(p);
    TEST_ASSERT_NOT_EQUAL(0, code);
    bebopc_process_free(p);
  }
}

void test_process_null_safety(void);

void test_process_null_safety(void)
{
  TEST_ASSERT_NULL(bebopc_process_spawn(NULL));
  TEST_ASSERT_FALSE(bebopc_process_write(NULL, "data", 4));
  bebopc_process_close_stdin(NULL);
  TEST_ASSERT_NULL(bebopc_process_read_all(NULL, NULL));
  TEST_ASSERT_EQUAL(-1, bebopc_process_wait(NULL));
  bebopc_process_free(NULL);
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_runner_init_null_runner);
  RUN_TEST(test_runner_init_null_ctx);
  RUN_TEST(test_runner_init_null_beb_ctx);
  RUN_TEST(test_runner_init_null_desc);
  RUN_TEST(test_runner_init_files_count_mismatch);

  RUN_TEST(test_runner_init_cleanup);
  RUN_TEST(test_runner_init_with_files);

  RUN_TEST(test_find_plugin_not_found);
  RUN_TEST(test_find_plugin_null);

  RUN_TEST(test_process_spawn_not_found);
  RUN_TEST(test_process_null_safety);

  return UNITY_END();
}
