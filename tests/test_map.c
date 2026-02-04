#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "bebop_wire.h"
#include "unity.h"

#ifdef __clang__
#pragma clang diagnostic ignored "-Wgnu-statement-expression-from-macro-expansion"
#pragma clang diagnostic ignored "-Wmissing-field-initializers"
#elif defined(__GNUC__)
#pragma GCC diagnostic ignored "-Wpedantic"
#pragma GCC diagnostic ignored "-Wunused-value"
#pragma GCC diagnostic ignored "-Wmissing-braces"
#pragma GCC diagnostic ignored "-Wmissing-field-initializers"
#endif

static void* _alloc(void* ptr, size_t old, size_t new, void* ctx)
{
  (void)ctx;
  (void)old;
  if (new == 0) {
    free(ptr);
    return NULL;
  }
  return realloc(ptr, new);
}

// ============================================================================
// REAL-WORLD EXAMPLES
// ============================================================================

typedef struct {
  int32_t x, y;
} Vec2;

typedef struct {
  Bebop_Str name;
  int32_t health;
  int32_t score;
  Vec2 position;
} Player;

typedef struct {
  Bebop_Str key;
  Bebop_Str value;
} Header;

static Bebop_WireCtx* ctx;

void setUp(void)
{
  Bebop_WireCtxOpts opts = Bebop_WireCtx_DefaultOpts();
  opts.arena_options.allocator.alloc = _alloc;
  ctx = Bebop_WireCtx_New(&opts);
}

void tearDown(void)
{
  Bebop_WireCtx_Free(ctx);
}

// ============================================================================
// LOW-LEVEL SWISSTABLE MAP TESTS
// ============================================================================

static void test_map_init(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  TEST_ASSERT_EQUAL(0, m.length);
  TEST_ASSERT_EQUAL(0, m.capacity);
  TEST_ASSERT_NULL(m.ctrl);
  TEST_ASSERT_NULL(m.slots);
  TEST_ASSERT_EQUAL_PTR(Bebop_MapHash_I32, m.hash);
  TEST_ASSERT_EQUAL_PTR(Bebop_MapEq_I32, m.eq);
  TEST_ASSERT_EQUAL_PTR(ctx, m.ctx);
}

static void test_map_put_get_single(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  int32_t key = 42;
  int32_t val = 100;
  int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp = key;
  *vp = val;

  bool inserted = Bebop_Map_Put(&m, kp, vp);
  TEST_ASSERT_TRUE(inserted);
  TEST_ASSERT_EQUAL(1, m.length);

  int32_t* found = (int32_t*)Bebop_Map_Get(&m, &key);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL(100, *found);
}

static void test_map_put_get_multiple(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  for (int32_t i = 0; i < 10; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i * 10;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(10, m.length);

  for (int32_t i = 0; i < 10; i++) {
    int32_t* found = (int32_t*)Bebop_Map_Get(&m, &i);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL(i * 10, *found);
  }

  int32_t missing = 999;
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &missing));
}

static void test_map_overwrite(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  int32_t key = 42;
  int32_t* kp1 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp1 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp1 = key;
  *vp1 = 100;
  Bebop_Map_Put(&m, kp1, vp1);
  TEST_ASSERT_EQUAL(1, m.length);

  int32_t* kp2 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp2 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp2 = key;
  *vp2 = 200;
  bool success = Bebop_Map_Put(&m, kp2, vp2);

  TEST_ASSERT_TRUE(success);
  TEST_ASSERT_EQUAL(1, m.length);  // still only 1 entry

  int32_t* found = (int32_t*)Bebop_Map_Get(&m, &key);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL(200, *found);  // value was overwritten
}

static void test_map_delete(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  for (int32_t i = 0; i < 5; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i * 10;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(5, m.length);

  int32_t key = 2;
  bool deleted = Bebop_Map_Del(&m, &key);
  TEST_ASSERT_TRUE(deleted);
  TEST_ASSERT_EQUAL(4, m.length);
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &key));

  int32_t other = 3;
  TEST_ASSERT_NOT_NULL(Bebop_Map_Get(&m, &other));
}

static void test_map_delete_nonexistent(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp = 42;
  *vp = 100;
  Bebop_Map_Put(&m, kp, vp);

  int32_t missing = 999;
  bool deleted = Bebop_Map_Del(&m, &missing);
  TEST_ASSERT_FALSE(deleted);
  TEST_ASSERT_EQUAL(1, m.length);
}

static void test_map_clear(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  for (int32_t i = 0; i < 10; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(10, m.length);

  Bebop_Map_Clear(&m);
  TEST_ASSERT_EQUAL(0, m.length);

  int32_t key = 5;
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &key));
}

static void test_map_grow(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  size_t initial_cap = 0;
  for (int32_t i = 0; i < 100; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i * 10;
    Bebop_Map_Put(&m, kp, vp);

    if (i == 0) {
      initial_cap = m.capacity;
    }
  }

  TEST_ASSERT_EQUAL(100, m.length);
  TEST_ASSERT_TRUE(m.capacity > initial_cap);

  for (int32_t i = 0; i < 100; i++) {
    int32_t* found = (int32_t*)Bebop_Map_Get(&m, &i);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL(i * 10, *found);
  }
}

static void test_map_many_entries(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  const int count = 1000;
  for (int32_t i = 0; i < count; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i * 2;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(count, (int)m.length);

  for (int32_t i = 0; i < count; i++) {
    int32_t* found = (int32_t*)Bebop_Map_Get(&m, &i);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL(i * 2, *found);
  }
}

static void test_map_iterator(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  int32_t expected_sum = 0;
  for (int32_t i = 1; i <= 10; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i * 10;
    Bebop_Map_Put(&m, kp, vp);
    expected_sum += i * 10;
  }

  Bebop_MapIter it;
  Bebop_MapIter_Init(&it, &m);

  int32_t sum = 0;
  int count = 0;
  void* k;
  void* v;
  while (Bebop_MapIter_Next(&it, &k, &v)) {
    sum += *(int32_t*)v;
    count++;
  }

  TEST_ASSERT_EQUAL(10, count);
  TEST_ASSERT_EQUAL(expected_sum, sum);
}

static void test_map_iterator_empty(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  Bebop_MapIter it;
  Bebop_MapIter_Init(&it, &m);

  void* k;
  void* v;
  TEST_ASSERT_FALSE(Bebop_MapIter_Next(&it, &k, &v));
}

static void test_map_string_keys(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_STR(&m, ctx);

  const char* keys[] = {"alpha", "beta", "gamma", "delta", "epsilon"};
  for (int i = 0; i < 5; i++) {
    Bebop_Str* kp = Bebop_WireCtx_Alloc(ctx, sizeof(Bebop_Str));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    kp->data = keys[i];
    kp->length = (uint32_t)strlen(keys[i]);
    *vp = i + 1;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(5, m.length);

  Bebop_Str lookup = {.data = "gamma", .length = 5};
  int32_t* found = (int32_t*)Bebop_Map_Get(&m, &lookup);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL(3, *found);

  Bebop_Str missing = {.data = "omega", .length = 5};
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &missing));
}

static void test_map_uuid_keys(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_UUID(&m, ctx);

  Bebop_UUID uuids[3] = {
      {{0x01,
        0x02,
        0x03,
        0x04,
        0x05,
        0x06,
        0x07,
        0x08,
        0x09,
        0x0a,
        0x0b,
        0x0c,
        0x0d,
        0x0e,
        0x0f,
        0x10}},
      {{0x11,
        0x12,
        0x13,
        0x14,
        0x15,
        0x16,
        0x17,
        0x18,
        0x19,
        0x1a,
        0x1b,
        0x1c,
        0x1d,
        0x1e,
        0x1f,
        0x20}},
      {{0x21,
        0x22,
        0x23,
        0x24,
        0x25,
        0x26,
        0x27,
        0x28,
        0x29,
        0x2a,
        0x2b,
        0x2c,
        0x2d,
        0x2e,
        0x2f,
        0x30}},
  };

  for (int i = 0; i < 3; i++) {
    Bebop_UUID* kp = Bebop_WireCtx_Alloc(ctx, sizeof(Bebop_UUID));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = uuids[i];
    *vp = (i + 1) * 100;
    Bebop_Map_Put(&m, kp, vp);
  }

  TEST_ASSERT_EQUAL(3, m.length);

  int32_t* found = (int32_t*)Bebop_Map_Get(&m, &uuids[1]);
  TEST_ASSERT_NOT_NULL(found);
  TEST_ASSERT_EQUAL(200, *found);

  Bebop_UUID missing = {
      {0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff}
  };
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &missing));
}

static void test_map_delete_and_reinsert(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  int32_t key = 42;
  int32_t* kp1 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp1 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp1 = key;
  *vp1 = 100;
  Bebop_Map_Put(&m, kp1, vp1);

  TEST_ASSERT_EQUAL(100, *(int32_t*)Bebop_Map_Get(&m, &key));

  Bebop_Map_Del(&m, &key);
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, &key));
  TEST_ASSERT_EQUAL(0, m.length);

  int32_t* kp2 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  int32_t* vp2 = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
  *kp2 = key;
  *vp2 = 200;
  Bebop_Map_Put(&m, kp2, vp2);

  TEST_ASSERT_EQUAL(1, m.length);
  TEST_ASSERT_EQUAL(200, *(int32_t*)Bebop_Map_Get(&m, &key));
}

static void test_map_delete_during_probe(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  for (int32_t i = 0; i < 20; i++) {
    int32_t* kp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    int32_t* vp = Bebop_WireCtx_Alloc(ctx, sizeof(int32_t));
    *kp = i;
    *vp = i;
    Bebop_Map_Put(&m, kp, vp);
  }

  for (int32_t i = 0; i < 20; i += 2) {
    Bebop_Map_Del(&m, &i);
  }

  TEST_ASSERT_EQUAL(10, m.length);

  for (int32_t i = 1; i < 20; i += 2) {
    int32_t* found = (int32_t*)Bebop_Map_Get(&m, &i);
    TEST_ASSERT_NOT_NULL(found);
    TEST_ASSERT_EQUAL(i, *found);
  }
}

static void test_map_null_safety(void)
{
  Bebop_Map m = {0};

  TEST_ASSERT_NULL(Bebop_Map_Get(NULL, NULL));
  TEST_ASSERT_NULL(Bebop_Map_Get(&m, NULL));
  TEST_ASSERT_FALSE(Bebop_Map_Put(NULL, NULL, NULL));
  TEST_ASSERT_FALSE(Bebop_Map_Del(NULL, NULL));

  Bebop_MapIter it;
  Bebop_MapIter_Init(&it, NULL);
  void* k;
  void* v;
  TEST_ASSERT_FALSE(Bebop_MapIter_Next(&it, &k, &v));
}

// ============================================================================
// MACRO-BASED MAP TESTS (REAL-WORLD EXAMPLES)
// ============================================================================
#if !defined(_MSC_VER)

// ----------------------------------------------------------------------------
// Example: Game leaderboard - player scores by name
// ----------------------------------------------------------------------------
static void test_leaderboard(void)
{
  Bebop_Map scores;
  BBM_INIT(&scores, ctx, Bebop_Str);  // generic init from key type

  BBM_BEGIN(&scores, ctx, Bebop_Str, int32_t)
  // Record scores
  BBM_PUT("alice", 1500);
  BBM_PUT("bob", 2300);
  BBM_PUT("charlie", 1800);

  // Check high score
  TEST_ASSERT_EQUAL(2300, *BBM_GET("bob"));

  // Update score
  BBM_PUT("alice", 2500);
  TEST_ASSERT_EQUAL(2500, *BBM_GET("alice"));

  // Check if player exists before showing stats
  TEST_ASSERT_TRUE(BBM_HAS("charlie"));
  TEST_ASSERT_FALSE(BBM_HAS("dave"));

  // Sum all scores
  int32_t total = 0;
  BBM_EACH(name, score)
  {
    (void)name;
    total += *score;
  }
  TEST_ASSERT_EQUAL(2500 + 2300 + 1800, total);
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: Config/settings store
// ----------------------------------------------------------------------------
static void test_config_store(void)
{
  Bebop_Map config;
  BEBOP_MAP_INIT_STR(&config, ctx);

  BBM_BEGIN(&config, ctx, Bebop_Str, int32_t)
  BBM_PUT("max_connections", 100);
  BBM_PUT("timeout_ms", 5000);
  BBM_PUT("retry_count", 3);
  BBM_PUT("debug", 1);

  // Read config with defaults
  int32_t* timeout = BBM_GET("timeout_ms");
  TEST_ASSERT_EQUAL(5000, *timeout);

  // Check optional config
  int32_t* missing = BBM_GET("not_set");
  TEST_ASSERT_NULL(missing);

  TEST_ASSERT_EQUAL(4, BBM_LEN());
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: Entity component system - components by entity ID
// ----------------------------------------------------------------------------
static void test_entity_positions(void)
{
  Bebop_Map positions;
  BBM_INIT(&positions, ctx, int32_t);  // int32 keys

  BBM_BEGIN(&positions, ctx, int32_t, Vec2)
  // Entity IDs as keys, positions as values
  BBM_PUT(1001, {.x = 0, .y = 0});
  BBM_PUT(1002, {.x = 100, .y = 50});
  BBM_PUT(1003, {.x = -20, .y = 80});

  // Move entity 1001
  Vec2* pos = BBM_GET(1001);
  pos->x += 10;
  pos->y += 5;

  TEST_ASSERT_EQUAL(10, BBM_GET(1001)->x);
  TEST_ASSERT_EQUAL(5, BBM_GET(1001)->y);

  // Despawn entity
  BBM_DEL(1002);
  TEST_ASSERT_FALSE(BBM_HAS(1002));
  TEST_ASSERT_EQUAL(2, BBM_LEN());
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: HTTP headers
// ----------------------------------------------------------------------------
static void test_http_headers(void)
{
  Bebop_Map headers;
  BEBOP_MAP_INIT_STR(&headers, ctx);

  BBM_BEGIN(&headers, ctx, Bebop_Str, Bebop_Str)
  BBM_PUT("Content-Type", BBM_STR("application/json"));
  BBM_PUT("Authorization", BBM_STR("Bearer xyz123"));
  BBM_PUT("X-Request-ID", BBM_STR("req-456"));

  Bebop_Str* content_type = BBM_GET("Content-Type");
  TEST_ASSERT_EQUAL_STRING_LEN("application/json", content_type->data, content_type->length);

  // Check for optional header
  TEST_ASSERT_FALSE(BBM_HAS("X-Custom"));
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: UUID-keyed session store
// ----------------------------------------------------------------------------
static void test_session_store(void)
{
  Bebop_Map sessions;
  BBM_INIT(&sessions, ctx, Bebop_UUID);  // UUID keys

  Bebop_UUID session1 = {{0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0, 0, 0, 0, 0, 0, 0, 1}};
  Bebop_UUID session2 = {{0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0, 0, 0, 0, 0, 0, 0, 2}};

  BBM_BEGIN(&sessions, ctx, Bebop_UUID, Player)
  BBM_PUT(session1, {.name = BBM_STR("alice"), .health = 100, .score = 0, .position = {0, 0}});

  BBM_PUT(session2, {.name = BBM_STR("bob"), .health = 85, .score = 1500, .position = {100, 50}});

  // Update player state
  Player* p = BBM_GET(session1);
  p->health -= 10;
  p->score += 100;

  TEST_ASSERT_EQUAL(90, BBM_GET(session1)->health);
  TEST_ASSERT_EQUAL(100, BBM_GET(session1)->score);

  // Session logout
  BBM_DEL(session2);
  TEST_ASSERT_EQUAL(1, BBM_LEN());
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: Word frequency counter
// ----------------------------------------------------------------------------
static void test_word_frequency(void)
{
  Bebop_Map freq;
  BEBOP_MAP_INIT_STR(&freq, ctx);

  const char* words[] = {"the", "quick", "brown", "fox", "the", "lazy", "the"};
  size_t n = sizeof(words) / sizeof(words[0]);

  BBM_BEGIN(&freq, ctx, Bebop_Str, int32_t)
  for (size_t i = 0; i < n; i++) {
    int32_t* count = BBM_GET(words[i]);
    if (count) {
      (*count)++;
    } else {
      BBM_PUT(words[i], 1);
    }
  }

  TEST_ASSERT_EQUAL(3, *BBM_GET("the"));
  TEST_ASSERT_EQUAL(1, *BBM_GET("fox"));
  TEST_ASSERT_EQUAL(5, BBM_LEN());  // unique words
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: Caching expensive computations
// ----------------------------------------------------------------------------
static int32_t expensive_compute(int32_t n)
{
  return n * n;
}

static void test_compute_cache(void)
{
  Bebop_Map cache;
  BEBOP_MAP_INIT_I32(&cache, ctx);

  BBM_BEGIN(&cache, ctx, int32_t, int32_t)
  // Cache miss - compute and store
  int32_t* cached = BBM_GET(42);
  if (!cached) {
    BBM_PUT(42, expensive_compute(42));
    cached = BBM_GET(42);
  }
  TEST_ASSERT_EQUAL(1764, *cached);

  // Cache hit
  TEST_ASSERT_NOT_NULL(BBM_GET(42));
  TEST_ASSERT_EQUAL(1764, *BBM_GET(42));

  // Different key
  BBM_PUT(10, expensive_compute(10));
  TEST_ASSERT_EQUAL(100, *BBM_GET(10));
  BBM_END()
}

// ----------------------------------------------------------------------------
// Iterate outside block with BBM_FOREACH
// ----------------------------------------------------------------------------
static void test_iterate_outside_block(void)
{
  Bebop_Map m;
  BEBOP_MAP_INIT_I32(&m, ctx);

  BBM_BEGIN(&m, ctx, int32_t, int32_t)
  BBM_PUT(1, 10);
  BBM_PUT(2, 20);
  BBM_PUT(3, 30);
  BBM_END()

  // Iterate outside the block - need explicit types
  int32_t sum = 0;
  BBM_FOREACH(&m, int32_t, k, int32_t, v)
  {
    (void)k;
    sum += *v;
  }
  TEST_ASSERT_EQUAL(60, sum);
}

// ----------------------------------------------------------------------------
// Example: Access map from decoded message struct
// ----------------------------------------------------------------------------

// Simulated decoded message with a map field
typedef struct {
  Bebop_Str name;
  Bebop_Map inventory;  // map<string, int32> - item name -> quantity
} PlayerInventory;

static void test_decoded_struct_map(void)
{
  // Simulate a decoded message - in real code this comes from Decode()
  PlayerInventory msg;
  msg.name = BBM_STR("alice");
  BEBOP_MAP_INIT_STR(&msg.inventory, ctx);

  // Populate as if decoded (in real code, decoder fills this)
  BBM_BEGIN(&msg.inventory, ctx, Bebop_Str, int32_t)
  BBM_PUT("sword", 1);
  BBM_PUT("potion", 5);
  BBM_PUT("gold", 100);
  BBM_END()

  // --- Now the real usage: type-safe access to decoded map ---

  BBM_BEGIN(&msg.inventory, ctx, Bebop_Str, int32_t)
  // Direct lookup
  int32_t* gold = BBM_GET("gold");
  TEST_ASSERT_EQUAL(100, *gold);

  // Check before access
  if (BBM_HAS("potion")) {
    int32_t* potions = BBM_GET("potion");
    *potions -= 1;  // use a potion
  }
  TEST_ASSERT_EQUAL(4, *BBM_GET("potion"));

  // Iterate all items
  int32_t total_items = 0;
  BBM_EACH(item_name, quantity)
  {
    (void)item_name;
    total_items += *quantity;
  }
  TEST_ASSERT_EQUAL(1 + 4 + 100, total_items);
  BBM_END()
}

// ----------------------------------------------------------------------------
// Example: Nested struct values from decoded map
// ----------------------------------------------------------------------------

typedef struct {
  int32_t level;
  int32_t damage;
  Bebop_Str element;
} WeaponStats;

typedef struct {
  Bebop_Map weapons;  // map<string, WeaponStats>
} Loadout;

static void test_decoded_map_with_struct_values(void)
{
  Loadout loadout;
  BEBOP_MAP_INIT_STR(&loadout.weapons, ctx);

  // Simulate decoded data
  BBM_BEGIN(&loadout.weapons, ctx, Bebop_Str, WeaponStats)
  BBM_PUT("excalibur", {.level = 10, .damage = 50, .element = BBM_STR("holy")});
  BBM_PUT("frostbrand", {.level = 5, .damage = 30, .element = BBM_STR("ice")});
  BBM_END()

  // Type-safe access to struct fields
  BBM_BEGIN(&loadout.weapons, ctx, Bebop_Str, WeaponStats)
  WeaponStats* sword = BBM_GET("excalibur");
  TEST_ASSERT_EQUAL(10, sword->level);
  TEST_ASSERT_EQUAL(50, sword->damage);

  // Find strongest weapon
  WeaponStats* strongest = NULL;
  BBM_EACH(name, stats)
  {
    (void)name;
    if (!strongest || stats->damage > strongest->damage) {
      strongest = stats;
    }
  }
  TEST_ASSERT_EQUAL(50, strongest->damage);
  BBM_END()
}
#endif

// ============================================================================
// MAIN
// ============================================================================

int main(void)
{
  UNITY_BEGIN();

  // Low-level SwissTable map tests
  RUN_TEST(test_map_init);
  RUN_TEST(test_map_put_get_single);
  RUN_TEST(test_map_put_get_multiple);
  RUN_TEST(test_map_overwrite);
  RUN_TEST(test_map_delete);
  RUN_TEST(test_map_delete_nonexistent);
  RUN_TEST(test_map_clear);
  RUN_TEST(test_map_grow);
  RUN_TEST(test_map_many_entries);
  RUN_TEST(test_map_iterator);
  RUN_TEST(test_map_iterator_empty);
  RUN_TEST(test_map_string_keys);
  RUN_TEST(test_map_uuid_keys);
  RUN_TEST(test_map_delete_and_reinsert);
  RUN_TEST(test_map_delete_during_probe);
  RUN_TEST(test_map_null_safety);

  // Macro-based map tests
#if !defined(_MSC_VER)
  RUN_TEST(test_leaderboard);
  RUN_TEST(test_config_store);
  RUN_TEST(test_entity_positions);
  RUN_TEST(test_http_headers);
  RUN_TEST(test_session_store);
  RUN_TEST(test_word_frequency);
  RUN_TEST(test_compute_cache);
  RUN_TEST(test_iterate_outside_block);
  RUN_TEST(test_decoded_struct_map);
  RUN_TEST(test_decoded_map_with_struct_values);
#endif

  return UNITY_END();
}
