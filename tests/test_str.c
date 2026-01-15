#include "bebop.c"
#include "test_common.h"
#include "unity.h"

static bebop_arena_t arena;
static bebop_intern_t intern;
static bebop_host_allocator_t alloc;

void setUp(void);
void tearDown(void);

void setUp(void)
{
  memset(&arena, 0, sizeof(arena));
  memset(&intern, 0, sizeof(intern));
  alloc = test_allocator();
  TEST_ASSERT_TRUE(bebop_arena_init(&arena, &alloc, 0));
  TEST_ASSERT_TRUE(bebop_intern_init(&intern, &arena, 0));
}

void tearDown(void)
{
  bebop_arena_destroy(&arena);
}

void test_intern_null_string(void);
void test_intern_empty_string(void);
void test_intern_basic(void);
void test_intern_same_string_twice(void);
void test_intern_different_strings(void);
void test_intern_str_get(void);
void test_intern_str_len(void);
void test_intern_str_eq(void);
void test_intern_str_is_null(void);
void test_intern_many_strings(void);
void test_intern_long_string(void);
void test_intern_binary_data(void);
void test_hash_fnv1a_basic(void);
void test_hash_fnv1a_empty(void);
void test_hash_fnv1a_different_strings(void);

void test_intern_null_string(void)
{
  bebop_str_t handle = bebop_intern(&intern, NULL);
  TEST_ASSERT_TRUE(bebop_str_is_null(handle));
}

void test_intern_empty_string(void)
{
  bebop_str_t handle = bebop_intern(&intern, "");
  TEST_ASSERT_FALSE(bebop_str_is_null(handle));
  TEST_ASSERT_EQUAL_STRING("", bebop_str_get(&intern, handle));
  TEST_ASSERT_EQUAL_UINT(0, bebop_str_len(&intern, handle));
}

void test_intern_basic(void)
{
  bebop_str_t handle = bebop_intern(&intern, "hello");
  TEST_ASSERT_FALSE(bebop_str_is_null(handle));
  TEST_ASSERT_TRUE(handle.idx > 0);
}

void test_intern_same_string_twice(void)
{
  bebop_str_t h1 = bebop_intern(&intern, "foo");
  bebop_str_t h2 = bebop_intern(&intern, "foo");

  TEST_ASSERT_FALSE(bebop_str_is_null(h1));
  TEST_ASSERT_FALSE(bebop_str_is_null(h2));
  TEST_ASSERT_TRUE(bebop_str_eq(h1, h2));
  TEST_ASSERT_EQUAL_UINT32(h1.idx, h2.idx);
}

void test_intern_different_strings(void)
{
  bebop_str_t h1 = bebop_intern(&intern, "foo");
  bebop_str_t h2 = bebop_intern(&intern, "bar");
  bebop_str_t h3 = bebop_intern(&intern, "baz");

  TEST_ASSERT_FALSE(bebop_str_eq(h1, h2));
  TEST_ASSERT_FALSE(bebop_str_eq(h2, h3));
  TEST_ASSERT_FALSE(bebop_str_eq(h1, h3));
}

void test_intern_str_get(void)
{
  bebop_str_t handle = bebop_intern(&intern, "hello world");
  const char* str = bebop_str_get(&intern, handle);

  TEST_ASSERT_NOT_NULL(str);
  TEST_ASSERT_EQUAL_STRING("hello world", str);
}

void test_intern_str_len(void)
{
  bebop_str_t handle = bebop_intern(&intern, "hello");
  size_t len = bebop_str_len(&intern, handle);

  TEST_ASSERT_EQUAL_UINT(5, len);
}

void test_intern_str_eq(void)
{
  bebop_str_t h1 = bebop_intern(&intern, "test");
  bebop_str_t h2 = bebop_intern(&intern, "test");
  bebop_str_t h3 = bebop_intern(&intern, "other");

  TEST_ASSERT_TRUE(bebop_str_eq(h1, h2));
  TEST_ASSERT_FALSE(bebop_str_eq(h1, h3));

  TEST_ASSERT_TRUE(bebop_str_eq(BEBOP_STR_NULL, BEBOP_STR_NULL));
  TEST_ASSERT_FALSE(bebop_str_eq(h1, BEBOP_STR_NULL));
}

void test_intern_str_is_null(void)
{
  bebop_str_t null_handle = BEBOP_STR_NULL;
  bebop_str_t valid_handle = bebop_intern(&intern, "test");

  TEST_ASSERT_TRUE(bebop_str_is_null(null_handle));
  TEST_ASSERT_FALSE(bebop_str_is_null(valid_handle));
}

void test_intern_many_strings(void)
{
  char buf[32];
  bebop_str_t handles[500];

  for (int i = 0; i < 500; i++) {
    snprintf(buf, sizeof(buf), "string_%d", i);
    handles[i] = bebop_intern(&intern, buf);
    TEST_ASSERT_FALSE(bebop_str_is_null(handles[i]));
  }

  for (int i = 0; i < 500; i++) {
    snprintf(buf, sizeof(buf), "string_%d", i);
    const char* str = bebop_str_get(&intern, handles[i]);
    TEST_ASSERT_EQUAL_STRING(buf, str);
  }

  for (int i = 0; i < 500; i++) {
    snprintf(buf, sizeof(buf), "string_%d", i);
    bebop_str_t h2 = bebop_intern(&intern, buf);
    TEST_ASSERT_TRUE(bebop_str_eq(handles[i], h2));
  }
}

void test_intern_long_string(void)
{
  char long_str[1024];
  memset(long_str, 'x', sizeof(long_str) - 1);
  long_str[sizeof(long_str) - 1] = '\0';

  bebop_str_t handle = bebop_intern(&intern, long_str);
  TEST_ASSERT_FALSE(bebop_str_is_null(handle));

  const char* retrieved = bebop_str_get(&intern, handle);
  TEST_ASSERT_EQUAL_STRING(long_str, retrieved);
  TEST_ASSERT_EQUAL_UINT(sizeof(long_str) - 1, bebop_str_len(&intern, handle));
}

void test_intern_binary_data(void)
{
  const char data[] = {'a', 'b', '\0', 'c', 'd'};
  bebop_str_t handle = bebop_intern_n(&intern, data, sizeof(data));

  TEST_ASSERT_FALSE(bebop_str_is_null(handle));
  TEST_ASSERT_EQUAL_UINT(sizeof(data), bebop_str_len(&intern, handle));

  const char* retrieved = bebop_str_get(&intern, handle);
  TEST_ASSERT_EQUAL_MEMORY(data, retrieved, sizeof(data));
}

void test_hash_fnv1a_basic(void)
{
  uint64_t h1 = bebop_hash_fnv1a("hello", 5);
  uint64_t h2 = bebop_hash_fnv1a("hello", 5);

  TEST_ASSERT_EQUAL_UINT64(h1, h2);
  TEST_ASSERT_NOT_EQUAL(0, h1);
}

void test_hash_fnv1a_empty(void)
{
  uint64_t h = bebop_hash_fnv1a("", 0);

  TEST_ASSERT_EQUAL_UINT64(0xcbf29ce484222325ULL, h);
}

void test_hash_fnv1a_different_strings(void)
{
  uint64_t h1 = bebop_hash_fnv1a("foo", 3);
  uint64_t h2 = bebop_hash_fnv1a("bar", 3);
  uint64_t h3 = bebop_hash_fnv1a("foobar", 6);

  TEST_ASSERT_NOT_EQUAL(h1, h2);
  TEST_ASSERT_NOT_EQUAL(h1, h3);
  TEST_ASSERT_NOT_EQUAL(h2, h3);
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_intern_null_string);
  RUN_TEST(test_intern_empty_string);

  RUN_TEST(test_intern_basic);
  RUN_TEST(test_intern_same_string_twice);
  RUN_TEST(test_intern_different_strings);

  RUN_TEST(test_intern_str_get);
  RUN_TEST(test_intern_str_len);
  RUN_TEST(test_intern_str_eq);
  RUN_TEST(test_intern_str_is_null);

  RUN_TEST(test_intern_many_strings);
  RUN_TEST(test_intern_long_string);
  RUN_TEST(test_intern_binary_data);

  RUN_TEST(test_hash_fnv1a_basic);
  RUN_TEST(test_hash_fnv1a_empty);
  RUN_TEST(test_hash_fnv1a_different_strings);

  return UNITY_END();
}
