#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bebopc_dir.h"
#include "unity.h"

static bebopc_ctx_t ctx;

void setUp(void);
void tearDown(void);

void setUp(void)
{
  bebopc_ctx_init(&ctx);
}

void tearDown(void)
{
  bebopc_ctx_cleanup(&ctx);
}

void test_dir_open_current(void);
void test_dir_open_not_found(void);
void test_dir_iterate(void);
void test_dir_sorted(void);
void test_file_exists(void);
void test_file_is_dir(void);
void test_file_is_file(void);
void test_path_join(void);
void test_path_dirname(void);
void test_path_basename(void);
void test_path_extension(void);
void test_path_is_absolute(void);
void test_getcwd(void);
void test_file_read_not_found(void);

void test_dir_open_current(void)
{
  bebopc_dir_t dir;
  bebopc_error_code_t result = bebopc_dir_open(&dir, &ctx, BEBOPC_STRING("."));
  TEST_ASSERT_EQUAL(BEBOPC_OK, result);
  TEST_ASSERT_TRUE(dir.has_next);
  bebopc_dir_close(&dir);
}

void test_dir_open_not_found(void)
{
  bebopc_dir_t dir;
  bebopc_error_code_t result =
      bebopc_dir_open(&dir, &ctx, BEBOPC_STRING("/nonexistent_path_12345"));
  TEST_ASSERT_EQUAL(BEBOPC_ERR_NOT_FOUND, result);
  TEST_ASSERT_TRUE(bebopc_error_has(&ctx.errors));

  const bebopc_error_t* err = bebopc_error_last(&ctx.errors);
  TEST_ASSERT_NOT_NULL(err);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_NOT_FOUND, err->code);

  bebopc_error_clear(&ctx.errors);
}

void test_dir_iterate(void)
{
  bebopc_dir_t dir;
  bebopc_error_code_t result = bebopc_dir_open(&dir, &ctx, BEBOPC_STRING("."));
  TEST_ASSERT_EQUAL(BEBOPC_OK, result);

  int count = 0;
  bebopc_file_t file;
  while (dir.has_next) {
    if (bebopc_dir_readfile(&dir, &file) == BEBOPC_OK) {
      TEST_ASSERT_TRUE(bebopc_strlen(file.name) > 0);
      TEST_ASSERT_TRUE(bebopc_strlen(file.path) > 0);
      count++;
    }
    bebopc_dir_next(&dir);
  }

  TEST_ASSERT_TRUE(count > 0);
  bebopc_dir_close(&dir);
}

void test_dir_sorted(void)
{
  bebopc_dir_t dir;
  bebopc_error_code_t result = bebopc_dir_open_sorted(&dir, &ctx, BEBOPC_STRING("."));
  TEST_ASSERT_EQUAL(BEBOPC_OK, result);
  TEST_ASSERT_TRUE(dir.n_files > 0);

  bebopc_file_t file;
  TEST_ASSERT_EQUAL(BEBOPC_OK, bebopc_dir_readfile_n(&dir, &file, 0));
  TEST_ASSERT_TRUE(bebopc_strlen(file.name) > 0);

  bebopc_dir_close(&dir);
}

void test_file_exists(void)
{
  TEST_ASSERT_TRUE(bebopc_file_exists(BEBOPC_STRING(".")));
  TEST_ASSERT_FALSE(bebopc_file_exists(BEBOPC_STRING("/nonexistent_12345")));
}

void test_file_is_dir(void)
{
  TEST_ASSERT_TRUE(bebopc_file_is_dir(BEBOPC_STRING(".")));
  TEST_ASSERT_FALSE(bebopc_file_is_dir(BEBOPC_STRING("/nonexistent_12345")));
}

void test_file_is_file(void)
{
  TEST_ASSERT_FALSE(bebopc_file_is_file(BEBOPC_STRING(".")));
}

void test_path_join(void)
{
  char* p1 = bebopc_path_join("/foo", "bar");
  TEST_ASSERT_NOT_NULL(p1);
  TEST_ASSERT_EQUAL_STRING("/foo/bar", p1);
  free(p1);

  char* p2 = bebopc_path_join("/foo/", "bar");
  TEST_ASSERT_NOT_NULL(p2);
  TEST_ASSERT_EQUAL_STRING("/foo/bar", p2);
  free(p2);

  char* p3 = bebopc_path_join(".", "file.txt");
  TEST_ASSERT_NOT_NULL(p3);
  TEST_ASSERT_EQUAL_STRING("./file.txt", p3);
  free(p3);
}

void test_path_dirname(void)
{
  char* d1 = bebopc_path_dirname("/foo/bar/baz");
  TEST_ASSERT_NOT_NULL(d1);
  TEST_ASSERT_EQUAL_STRING("/foo/bar", d1);
  free(d1);

  char* d2 = bebopc_path_dirname("/foo");
  TEST_ASSERT_NOT_NULL(d2);
  TEST_ASSERT_EQUAL_STRING("/", d2);
  free(d2);

  char* d3 = bebopc_path_dirname("file.txt");
  TEST_ASSERT_NOT_NULL(d3);
  TEST_ASSERT_EQUAL_STRING(".", d3);
  free(d3);
}

void test_path_basename(void)
{
  const char* b1 = bebopc_path_basename("/foo/bar/baz.txt");
  TEST_ASSERT_EQUAL_STRING("baz.txt", b1);

  const char* b2 = bebopc_path_basename("file.txt");
  TEST_ASSERT_EQUAL_STRING("file.txt", b2);

  const char* b3 = bebopc_path_basename("/foo/");
  TEST_ASSERT_EQUAL_STRING("", b3);
}

void test_path_extension(void)
{
  const char* e1 = bebopc_path_extension("file.txt");
  TEST_ASSERT_EQUAL_STRING("txt", e1);

  const char* e2 = bebopc_path_extension("file.tar.gz");
  TEST_ASSERT_EQUAL_STRING("gz", e2);

  const char* e3 = bebopc_path_extension("noext");
  TEST_ASSERT_EQUAL_STRING("", e3);

  const char* e4 = bebopc_path_extension(".hidden");
  TEST_ASSERT_EQUAL_STRING("hidden", e4);
}

void test_path_is_absolute(void)
{
#ifdef BEBOPC_WINDOWS
  TEST_ASSERT_TRUE(bebopc_path_is_absolute(BEBOPC_STRING("C:\\foo")));
  TEST_ASSERT_TRUE(bebopc_path_is_absolute(BEBOPC_STRING("\\foo")));
  TEST_ASSERT_FALSE(bebopc_path_is_absolute(BEBOPC_STRING("foo\\bar")));
#else
  TEST_ASSERT_TRUE(bebopc_path_is_absolute(BEBOPC_STRING("/foo/bar")));
  TEST_ASSERT_FALSE(bebopc_path_is_absolute(BEBOPC_STRING("foo/bar")));
  TEST_ASSERT_FALSE(bebopc_path_is_absolute(BEBOPC_STRING("./foo")));
#endif
}

void test_getcwd(void)
{
  char* cwd = bebopc_getcwd();
  TEST_ASSERT_NOT_NULL(cwd);
  TEST_ASSERT_TRUE(strlen(cwd) > 0);
  TEST_ASSERT_TRUE(bebopc_path_is_absolute(cwd));
  free(cwd);
}

void test_file_read_not_found(void)
{
  size_t size;
  char* data = bebopc_file_read(&ctx, "/nonexistent_file_12345.txt", &size);
  TEST_ASSERT_NULL(data);
  TEST_ASSERT_TRUE(bebopc_error_has(&ctx.errors));

  const bebopc_error_t* err = bebopc_error_last(&ctx.errors);
  TEST_ASSERT_EQUAL(BEBOPC_ERR_NOT_FOUND, err->code);

  bebopc_error_clear(&ctx.errors);
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_dir_open_current);
  RUN_TEST(test_dir_open_not_found);
  RUN_TEST(test_dir_iterate);
  RUN_TEST(test_dir_sorted);

  RUN_TEST(test_file_exists);
  RUN_TEST(test_file_is_dir);
  RUN_TEST(test_file_is_file);

  RUN_TEST(test_path_join);
  RUN_TEST(test_path_dirname);
  RUN_TEST(test_path_basename);
  RUN_TEST(test_path_extension);
  RUN_TEST(test_path_is_absolute);
  RUN_TEST(test_getcwd);

  RUN_TEST(test_file_read_not_found);

  return UNITY_END();
}
