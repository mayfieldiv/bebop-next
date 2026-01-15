#include "bebop.c"
#include "test_common.h"
#include "unity.h"

static bebop_arena_t arena;
static bebop_defmap map;
static bebop_host_allocator_t alloc;

void setUp(void);
void tearDown(void);

void setUp(void)
{
  memset(&arena, 0, sizeof(arena));
  alloc = test_allocator();
  bebop_arena_init(&arena, &alloc, 0);
  map = bebop_defmap_new(16, &arena);
}

void tearDown(void)
{
  bebop_defmap_destroy(&map);
  bebop_arena_destroy(&arena);
}

void test_map_new(void);
void test_map_insert_find_single(void);
void test_map_insert_find_multiple(void);
void test_map_update_existing(void);
void test_map_find_nonexistent(void);
void test_map_contains(void);
void test_map_erase(void);
void test_map_clear(void);
void test_map_iteration(void);
void test_map_iteration_empty(void);
void test_map_grow(void);

void test_map_new(void)
{
  TEST_ASSERT_EQUAL_UINT64(0, bebop_defmap_size(&map));
  TEST_ASSERT_TRUE(bebop_defmap_empty(&map));
}

void test_map_insert_find_single(void)
{
  int value = 42;
  uint32_t key = 1;

  bebop_defmap_Entry entry = {key, &value};
  bebop_defmap_insert(&map, &entry);

  TEST_ASSERT_EQUAL_UINT64(1, bebop_defmap_size(&map));

  bebop_defmap_Iter it = bebop_defmap_find(&map, &key);
  bebop_defmap_Entry* found = bebop_defmap_Iter_get(&it);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL_UINT32(key, found->key);
  TEST_ASSERT_EQUAL_INT(42, *(int*)found->val);
}

void test_map_insert_find_multiple(void)
{
  int values[5] = {10, 20, 30, 40, 50};

  for (uint32_t i = 0; i < 5; i++) {
    bebop_defmap_Entry entry = {i + 1, &values[i]};
    bebop_defmap_insert(&map, &entry);
  }

  TEST_ASSERT_EQUAL_UINT64(5, bebop_defmap_size(&map));

  for (uint32_t i = 0; i < 5; i++) {
    uint32_t key = i + 1;
    bebop_defmap_Iter it = bebop_defmap_find(&map, &key);
    bebop_defmap_Entry* found = bebop_defmap_Iter_get(&it);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL_INT(values[i], *(int*)found->val);
  }
}

void test_map_update_existing(void)
{
  int value1 = 100;
  int value2 = 200;
  uint32_t key = 1;

  bebop_defmap_Entry entry1 = {key, &value1};
  bebop_defmap_insert(&map, &entry1);
  TEST_ASSERT_EQUAL_UINT64(1, bebop_defmap_size(&map));

  bebop_defmap_Entry entry2 = {key, &value2};
  bebop_defmap_Insert result = bebop_defmap_insert(&map, &entry2);

  TEST_ASSERT_FALSE(result.inserted);
  TEST_ASSERT_EQUAL_UINT64(1, bebop_defmap_size(&map));

  bebop_defmap_Iter it = bebop_defmap_find(&map, &key);
  bebop_defmap_Entry* found = bebop_defmap_Iter_get(&it);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL_INT(100, *(int*)found->val);
}

void test_map_find_nonexistent(void)
{
  int value = 42;
  uint32_t key = 1;

  bebop_defmap_Entry entry = {key, &value};
  bebop_defmap_insert(&map, &entry);

  uint32_t missing_key = 999;
  bebop_defmap_Iter it = bebop_defmap_find(&map, &missing_key);
  bebop_defmap_Entry* found = bebop_defmap_Iter_get(&it);
  TEST_ASSERT_NULL(found);
}

void test_map_contains(void)
{
  int value = 42;
  uint32_t key = 1;

  bebop_defmap_Entry entry = {key, &value};
  bebop_defmap_insert(&map, &entry);

  TEST_ASSERT_TRUE(bebop_defmap_contains(&map, &key));

  uint32_t missing = 2;
  TEST_ASSERT_FALSE(bebop_defmap_contains(&map, &missing));
}

void test_map_erase(void)
{
  int values[3] = {1, 2, 3};

  for (uint32_t i = 0; i < 3; i++) {
    bebop_defmap_Entry entry = {i + 1, &values[i]};
    bebop_defmap_insert(&map, &entry);
  }

  TEST_ASSERT_EQUAL_UINT64(3, bebop_defmap_size(&map));

  uint32_t key = 2;
  TEST_ASSERT_TRUE(bebop_defmap_erase(&map, &key));
  TEST_ASSERT_EQUAL_UINT64(2, bebop_defmap_size(&map));
  TEST_ASSERT_FALSE(bebop_defmap_contains(&map, &key));

  key = 1;
  TEST_ASSERT_TRUE(bebop_defmap_contains(&map, &key));
  key = 3;
  TEST_ASSERT_TRUE(bebop_defmap_contains(&map, &key));
}

void test_map_clear(void)
{
  int values[3] = {1, 2, 3};

  for (uint32_t i = 0; i < 3; i++) {
    bebop_defmap_Entry entry = {i + 1, &values[i]};
    bebop_defmap_insert(&map, &entry);
  }

  TEST_ASSERT_EQUAL_UINT64(3, bebop_defmap_size(&map));

  bebop_defmap_clear(&map);

  TEST_ASSERT_EQUAL_UINT64(0, bebop_defmap_size(&map));
  TEST_ASSERT_TRUE(bebop_defmap_empty(&map));
}

void test_map_iteration(void)
{
  int values[5] = {10, 20, 30, 40, 50};

  for (uint32_t i = 0; i < 5; i++) {
    bebop_defmap_Entry entry = {i + 1, &values[i]};
    bebop_defmap_insert(&map, &entry);
  }

  bool seen[6] = {false};
  uint32_t count = 0;

  bebop_defmap_Iter it = bebop_defmap_iter(&map);
  for (bebop_defmap_Entry* e = bebop_defmap_Iter_get(&it); e != NULL;
       e = bebop_defmap_Iter_next(&it))
  {
    TEST_ASSERT_TRUE(e->key >= 1 && e->key <= 5);
    TEST_ASSERT_FALSE(seen[e->key]);
    seen[e->key] = true;
    TEST_ASSERT_EQUAL_INT(values[e->key - 1], *(int*)e->val);
    count++;
  }

  TEST_ASSERT_EQUAL_UINT32(5, count);

  for (uint32_t i = 1; i <= 5; i++) {
    TEST_ASSERT_TRUE(seen[i]);
  }
}

void test_map_iteration_empty(void)
{
  bebop_defmap_Iter it = bebop_defmap_iter(&map);
  bebop_defmap_Entry* e = bebop_defmap_Iter_get(&it);
  TEST_ASSERT_NULL(e);
}

void test_map_grow(void)
{
  int values[100];
  for (uint32_t i = 0; i < 100; i++) {
    values[i] = (int)(i * 10);
    bebop_defmap_Entry entry = {i + 1, &values[i]};
    bebop_defmap_insert(&map, &entry);
  }

  TEST_ASSERT_EQUAL_UINT64(100, bebop_defmap_size(&map));

  for (uint32_t i = 0; i < 100; i++) {
    uint32_t key = i + 1;
    bebop_defmap_Iter it = bebop_defmap_find(&map, &key);
    bebop_defmap_Entry* found = bebop_defmap_Iter_get(&it);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL_INT((int)(i * 10), *(int*)found->val);
  }
}

int main(void)
{
  UNITY_BEGIN();

  RUN_TEST(test_map_new);

  RUN_TEST(test_map_insert_find_single);
  RUN_TEST(test_map_insert_find_multiple);
  RUN_TEST(test_map_update_existing);
  RUN_TEST(test_map_find_nonexistent);
  RUN_TEST(test_map_contains);

  RUN_TEST(test_map_erase);
  RUN_TEST(test_map_clear);

  RUN_TEST(test_map_iteration);
  RUN_TEST(test_map_iteration_empty);

  RUN_TEST(test_map_grow);

  return UNITY_END();
}
