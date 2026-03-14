#include "bench_harness.h"
#include <cstring>
#include <fstream>
#include <functional>
#include <mutex>
#include <random>
#include <sstream>

static TestPerson g_small_person;
static TestPerson g_medium_person;
static TestOrder g_small_order;
static TestOrder g_large_order;
static TestEvent g_small_event;
static TestEvent g_large_event;
static TestTreeNode g_wide_tree;
static TestTreeNode g_deep_tree;
static TestJsonValue g_small_json;
static TestJsonValue g_large_json;
static TestJsonValue g_nested_json;
static TestDocument g_small_document;
static TestDocument g_large_document;
static TestChunkedText g_alice_chunks;
static TestEmbeddingBF16 g_embedding_384;
static TestEmbeddingBF16 g_embedding_768;
static TestEmbeddingBF16 g_embedding_1536;
static TestEmbeddingF32 g_embedding_f32_768;
static TestEmbeddingBatch g_embedding_batch;
static TestLLMStreamChunk g_llm_chunk_small;
static TestLLMStreamChunk g_llm_chunk_large;
static TestTensorShard g_tensor_shard_small;
static TestTensorShard g_tensor_shard_large;
static TestInferenceResponse g_inference_response;
static std::once_flag g_init_flag;

static std::string random_string(std::mt19937_64& rng, size_t len)
{
  static const char charset[] = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  std::string s(len, ' ');
  std::uniform_int_distribution<size_t> dist(0, sizeof(charset) - 2);
  for (size_t i = 0; i < len; i++) {
    s[i] = charset[dist(rng)];
  }
  return s;
}

static TestUUID random_uuid(std::mt19937_64& rng)
{
  TestUUID uuid;
  for (int i = 0; i < 16; i++) {
    uuid.bytes[i] = static_cast<uint8_t>(rng() & 0xFF);
  }
  uuid.bytes[6] = (uuid.bytes[6] & 0x0F) | 0x40;
  uuid.bytes[8] = (uuid.bytes[8] & 0x3F) | 0x80;
  return uuid;
}

static uint16_t float_to_bf16(float f)
{
  uint32_t bits;
  memcpy(&bits, &f, sizeof(bits));
  bits += 0x7FFF + ((bits >> 16) & 1);
  return static_cast<uint16_t>(bits >> 16);
}

static std::vector<uint16_t> random_bf16_vector(std::mt19937_64& rng, size_t dim)
{
  std::vector<uint16_t> v(dim);
  std::uniform_real_distribution<float> dist(-1.0f, 1.0f);
  for (size_t i = 0; i < dim; i++) {
    v[i] = float_to_bf16(dist(rng));
  }
  return v;
}

static std::vector<float> random_f32_vector(std::mt19937_64& rng, size_t dim)
{
  std::vector<float> v(dim);
  std::uniform_real_distribution<float> dist(-1.0f, 1.0f);
  for (size_t i = 0; i < dim; i++) {
    v[i] = dist(rng);
  }
  return v;
}

static void do_init_test_data()
{
  std::mt19937_64 rng(42);
  std::uniform_int_distribution<int32_t> age_dist(18, 80);
  std::uniform_int_distribution<int64_t> id_dist(1, 1000000);
  std::uniform_real_distribution<double> price_dist(10.0, 10000.0);

  g_small_person = {.id = 12345, .name = "John", .email = "j@x.co", .age = 30};

  g_medium_person = {
      .id = 987654321,
      .name = "Alexander Maximilian Henderson-Worthington III",
      .email = "alexander.henderson.worthington@enterprise-corporation.example.com",
      .age = age_dist(rng)
  };

  g_small_order = {
      .order_id = id_dist(rng),
      .customer_id = id_dist(rng),
      .item_ids = {101, 102, 103},
      .quantities = {1, 2, 1},
      .total = price_dist(rng),
      .timestamp = 1704067200000
  };

  g_large_order.order_id = id_dist(rng);
  g_large_order.customer_id = id_dist(rng);
  g_large_order.total = 0;
  g_large_order.timestamp = 1704067200000;
  for (int i = 0; i < 100; i++) {
    g_large_order.item_ids.push_back(id_dist(rng));
    int32_t qty = (rng() % 10) + 1;
    g_large_order.quantities.push_back(qty);
    g_large_order.total += price_dist(rng) * qty;
  }

  g_small_event = {
      .id = id_dist(rng),
      .type = "click",
      .source = "web",
      .timestamp = 1704067200000,
      .payload = {0x01, 0x02, 0x03, 0x04}
  };

  g_large_event.id = id_dist(rng);
  g_large_event.type = "data_transfer";
  g_large_event.source = "backend_service_cluster_node_42";
  g_large_event.timestamp = 1704067200000;
  g_large_event.payload.resize(4096);
  for (size_t i = 0; i < g_large_event.payload.size(); i++) {
    g_large_event.payload[i] = static_cast<uint8_t>(rng() & 0xFF);
  }

  g_wide_tree.value = 999;
  g_wide_tree.children.resize(100);
  for (int i = 0; i < 100; i++) {
    g_wide_tree.children[i].value = i;
  }

  std::function<void(TestTreeNode&, int, int)> build_deep =
      [&](TestTreeNode& node, int depth, int idx)
  {
    node.value = depth * 100 + idx;
    if (depth < 9) {
      node.children.resize(2);
      build_deep(node.children[0], depth + 1, idx * 2);
      build_deep(node.children[1], depth + 1, idx * 2 + 1);
    }
  };
  build_deep(g_deep_tree, 0, 0);

  g_small_json.type = TestJsonValue::Type::Object;
  g_small_json.object_val = {
      {"name", {TestJsonValue::Type::String, false, 0.0, "John Doe", {}, {}}},
      {"age", {TestJsonValue::Type::Number, false, 30.0, "", {}, {}}},
      {"active", {TestJsonValue::Type::Bool, true, 0.0, "", {}, {}}}
  };

  g_large_json.type = TestJsonValue::Type::Object;
  TestJsonValue items_array;
  items_array.type = TestJsonValue::Type::List;
  for (int i = 0; i < 20; i++) {
    TestJsonValue item;
    item.type = TestJsonValue::Type::Object;
    item.object_val = {
        {"id", {TestJsonValue::Type::Number, false, static_cast<double>(i + 1000), "", {}, {}}},
        {"name", {TestJsonValue::Type::String, false, 0.0, random_string(rng, 20), {}, {}}},
        {"price", {TestJsonValue::Type::Number, false, price_dist(rng), "", {}, {}}},
        {"in_stock", {TestJsonValue::Type::Bool, (i % 3) != 0, 0.0, "", {}, {}}}
    };
    items_array.list_val.push_back(item);
  }
  g_large_json.object_val = {
      {"status", {TestJsonValue::Type::String, false, 0.0, "success", {}, {}}},
      {"total_count", {TestJsonValue::Type::Number, false, 20.0, "", {}, {}}},
      {"items", items_array},
      {"pagination",
       {TestJsonValue::Type::Object,
        false,
        0.0,
        "",
        {},
        {{"page", {TestJsonValue::Type::Number, false, 1.0, "", {}, {}}},
         {"per_page", {TestJsonValue::Type::Number, false, 20.0, "", {}, {}}},
         {"has_more", {TestJsonValue::Type::Bool, true, 0.0, "", {}, {}}}}}}
  };

  // Nested JSON: list-of-lists to exercise recursive storage interleaving.
  // Structure: { "matrix": [[num,...] x10] x10, "deep": [[[[num]]]], "rows": [[{obj},...] x5] x5 }
  {
    g_nested_json.type = TestJsonValue::Type::Object;

    // 10x10 number matrix
    TestJsonValue matrix;
    matrix.type = TestJsonValue::Type::List;
    for (int r = 0; r < 10; r++) {
      TestJsonValue row;
      row.type = TestJsonValue::Type::List;
      for (int c = 0; c < 10; c++) {
        row.list_val.push_back(
            {TestJsonValue::Type::Number, false, static_cast<double>(r * 10 + c), "", {}, {}}
        );
      }
      matrix.list_val.push_back(row);
    }

    // 4-level deep nesting: [[[[ numbers ]]]]
    TestJsonValue deep;
    deep.type = TestJsonValue::Type::List;
    for (int a = 0; a < 3; a++) {
      TestJsonValue l2;
      l2.type = TestJsonValue::Type::List;
      for (int b = 0; b < 3; b++) {
        TestJsonValue l3;
        l3.type = TestJsonValue::Type::List;
        for (int c = 0; c < 3; c++) {
          TestJsonValue l4;
          l4.type = TestJsonValue::Type::List;
          for (int d = 0; d < 3; d++) {
            l4.list_val.push_back(
                {TestJsonValue::Type::Number, false, static_cast<double>(a * 27 + b * 9 + c * 3 + d),
                 "", {}, {}}
            );
          }
          l3.list_val.push_back(l4);
        }
        l2.list_val.push_back(l3);
      }
      deep.list_val.push_back(l2);
    }

    // 5x5 grid of objects (list of list-of-objects)
    TestJsonValue rows;
    rows.type = TestJsonValue::Type::List;
    for (int r = 0; r < 5; r++) {
      TestJsonValue row;
      row.type = TestJsonValue::Type::List;
      for (int c = 0; c < 5; c++) {
        TestJsonValue cell;
        cell.type = TestJsonValue::Type::Object;
        cell.object_val = {
            {"r", {TestJsonValue::Type::Number, false, static_cast<double>(r), "", {}, {}}},
            {"c", {TestJsonValue::Type::Number, false, static_cast<double>(c), "", {}, {}}},
            {"v", {TestJsonValue::Type::Number, false, price_dist(rng), "", {}, {}}}
        };
        row.list_val.push_back(cell);
      }
      rows.list_val.push_back(row);
    }

    g_nested_json.object_val = {{"matrix", matrix}, {"deep", deep}, {"rows", rows}};
  }

  g_small_document.title = "Hello World";
  g_small_document.body = "This is a test document.";

  g_large_document.title = "API Design Notes v2";
  g_large_document.body.resize(2000);
  for (size_t i = 0; i < g_large_document.body.size(); i++) {
    g_large_document.body[i] = 'a' + (i % 26);
  }
  g_large_document.metadata = {
      {"author", {TestJsonValue::Type::String, false, 0.0, "Engineering Team", {}, {}}},
      {"version", {TestJsonValue::Type::Number, false, 2.5, "", {}, {}}},
      {"tags",
       {TestJsonValue::Type::List,
        false,
        0.0,
        "",
        {{TestJsonValue::Type::String, false, 0.0, "spec", {}, {}},
         {TestJsonValue::Type::String, false, 0.0, "tech", {}, {}},
         {TestJsonValue::Type::String, false, 0.0, "api", {}, {}}},
        {}}}
  };

  std::ifstream file("../fixtures/Alice's Adventures in Wonderland.md");
  if (!file) {
    throw std::runtime_error(
        "Failed to open fixture: ../fixtures/Alice's Adventures in Wonderland.md"
    );
  }
  {
    std::ostringstream ss;
    ss << file.rdbuf();
    g_alice_chunks.source = ss.str();

    const std::string& src = g_alice_chunks.source;
    size_t pos = 0;
    size_t len = src.size();

    while (pos < len) {
      while (pos < len && (src[pos] == '\n' || src[pos] == '\r')) {
        pos++;
      }
      if (pos >= len) {
        break;
      }

      size_t start = pos;
      TestChunkKind kind = TestChunkKind::Paragraph;

      if (pos + 2 < len && src[pos] == '#' && src[pos + 1] == '#' && src[pos + 2] == ' ') {
        size_t line_end = src.find('\n', pos);
        if (line_end == std::string::npos) {
          line_end = len;
        }
        std::string line = src.substr(pos + 3, line_end - pos - 3);
        kind = (line.find("Chapter") == 0) ? TestChunkKind::Chapter : TestChunkKind::Heading;
        pos = line_end;
      } else {
        while (pos < len) {
          if (src[pos] == '\n') {
            if (pos + 1 < len && src[pos + 1] == '\n') {
              break;
            }
            if (pos + 2 < len && src[pos + 1] == '\r' && src[pos + 2] == '\n') {
              break;
            }
          }
          pos++;
        }
      }

      if (pos > start) {
        g_alice_chunks.spans.push_back(
            {static_cast<uint32_t>(start), static_cast<uint32_t>(pos - start), kind}
        );
      }
    }
  }

  g_embedding_384.id = random_uuid(rng);
  g_embedding_384.vector = random_bf16_vector(rng, 384);

  g_embedding_768.id = random_uuid(rng);
  g_embedding_768.vector = random_bf16_vector(rng, 768);

  g_embedding_1536.id = random_uuid(rng);
  g_embedding_1536.vector = random_bf16_vector(rng, 1536);

  g_embedding_f32_768.id = random_uuid(rng);
  g_embedding_f32_768.vector = random_f32_vector(rng, 768);

  g_embedding_batch.model = "text-embedding-3-small";
  g_embedding_batch.usage_tokens = 256;
  for (int i = 0; i < 8; i++) {
    TestEmbeddingBF16 emb;
    emb.id = random_uuid(rng);
    emb.vector = random_bf16_vector(rng, 1536);
    g_embedding_batch.embeddings.push_back(emb);
  }

  g_llm_chunk_small.chunk_id = 1;
  g_llm_chunk_small.tokens = {"Hello", ",", " world"};
  g_llm_chunk_small.finish_reason = "";
  for (const auto& tok : g_llm_chunk_small.tokens) {
    TestTokenAlternatives alts;
    alts.top_tokens.push_back({tok, static_cast<uint32_t>(rng() % 50000), -0.1f});
    g_llm_chunk_small.logprobs.push_back(alts);
  }

  g_llm_chunk_large.chunk_id = 42;
  g_llm_chunk_large.finish_reason = "stop";
  std::uniform_int_distribution<uint32_t> tok_dist(0, 50000);
  std::uniform_real_distribution<float> logp_dist(-5.0f, 0.0f);
  for (int i = 0; i < 32; i++) {
    g_llm_chunk_large.tokens.push_back(random_string(rng, 4));
    TestTokenAlternatives alts;
    for (int k = 0; k < 5; k++) {
      alts.top_tokens.push_back({random_string(rng, 4), tok_dist(rng), logp_dist(rng)});
    }
    g_llm_chunk_large.logprobs.push_back(alts);
  }

  g_tensor_shard_small.name = "model.layers.0.attention.wq.weight";
  g_tensor_shard_small.shape = {4096, 4096};
  g_tensor_shard_small.dtype = "bfloat16";
  g_tensor_shard_small.offset = 0;
  g_tensor_shard_small.total_elements = 4096 * 4096;
  g_tensor_shard_small.data = random_bf16_vector(rng, 1024);

  g_tensor_shard_large.name = "model.layers.31.feed_forward.w1.weight";
  g_tensor_shard_large.shape = {4096, 11008};
  g_tensor_shard_large.dtype = "bfloat16";
  g_tensor_shard_large.offset = 1024 * 1024;
  g_tensor_shard_large.total_elements = 4096 * 11008;
  g_tensor_shard_large.data = random_bf16_vector(rng, 32768);

  g_inference_response.request_id = random_uuid(rng);
  g_inference_response.timing.queue_time = {0, 1500000};
  g_inference_response.timing.inference_time = {0, 45000000};
  g_inference_response.timing.tokens_per_second = 2847.5f;
  for (int i = 0; i < 4; i++) {
    TestEmbeddingBF16 emb;
    emb.id = random_uuid(rng);
    emb.vector = random_bf16_vector(rng, 768);
    g_inference_response.embeddings.push_back(emb);
  }
}

static void init_test_data()
{
  std::call_once(g_init_flag, do_init_test_data);
}

const TestPerson& GetSmallPerson()
{
  init_test_data();
  return g_small_person;
}

const TestPerson& GetMediumPerson()
{
  init_test_data();
  return g_medium_person;
}

const TestOrder& GetSmallOrder()
{
  init_test_data();
  return g_small_order;
}

const TestOrder& GetLargeOrder()
{
  init_test_data();
  return g_large_order;
}

const TestEvent& GetSmallEvent()
{
  init_test_data();
  return g_small_event;
}

const TestEvent& GetLargeEvent()
{
  init_test_data();
  return g_large_event;
}

const TestTreeNode& GetWideTree()
{
  init_test_data();
  return g_wide_tree;
}

const TestTreeNode& GetDeepTree()
{
  init_test_data();
  return g_deep_tree;
}

const TestJsonValue& GetSmallJson()
{
  init_test_data();
  return g_small_json;
}

const TestJsonValue& GetLargeJson()
{
  init_test_data();
  return g_large_json;
}

const TestJsonValue& GetNestedJson()
{
  init_test_data();
  return g_nested_json;
}

const TestDocument& GetSmallDocument()
{
  init_test_data();
  return g_small_document;
}

const TestDocument& GetLargeDocument()
{
  init_test_data();
  return g_large_document;
}

const TestChunkedText& GetAliceChunks()
{
  init_test_data();
  return g_alice_chunks;
}

const TestEmbeddingBF16& GetEmbedding384()
{
  init_test_data();
  return g_embedding_384;
}

const TestEmbeddingBF16& GetEmbedding768()
{
  init_test_data();
  return g_embedding_768;
}

const TestEmbeddingBF16& GetEmbedding1536()
{
  init_test_data();
  return g_embedding_1536;
}

const TestEmbeddingF32& GetEmbeddingF32_768()
{
  init_test_data();
  return g_embedding_f32_768;
}

const TestEmbeddingBatch& GetEmbeddingBatch()
{
  init_test_data();
  return g_embedding_batch;
}

const TestLLMStreamChunk& GetLLMChunkSmall()
{
  init_test_data();
  return g_llm_chunk_small;
}

const TestLLMStreamChunk& GetLLMChunkLarge()
{
  init_test_data();
  return g_llm_chunk_large;
}

const TestTensorShard& GetTensorShardSmall()
{
  init_test_data();
  return g_tensor_shard_small;
}

const TestTensorShard& GetTensorShardLarge()
{
  init_test_data();
  return g_tensor_shard_large;
}

const TestInferenceResponse& GetInferenceResponse()
{
  init_test_data();
  return g_inference_response;
}
