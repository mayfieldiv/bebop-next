#include <benchmark/benchmark.h>

#include "bench_harness.h"
#include "simdjson.h"
#include <iomanip>
#include <sstream>
#include <string>

static simdjson::ondemand::parser g_parser;

static std::string escape_json_string(const std::string& s)
{
  std::ostringstream ss;
  for (char c : s) {
    if (c == '"') {
      ss << "\\\"";
    } else if (c == '\\') {
      ss << "\\\\";
    } else if (c == '\n') {
      ss << "\\n";
    } else if (c == '\r') {
      ss << "\\r";
    } else if (c == '\t') {
      ss << "\\t";
    } else {
      ss << c;
    }
  }
  return ss.str();
}

static std::string json_value_to_string(const TestJsonValue& v)
{
  std::ostringstream ss;
  switch (v.type) {
    case TestJsonValue::Type::Null:
      ss << "null";
      break;
    case TestJsonValue::Type::Bool:
      ss << (v.bool_val ? "true" : "false");
      break;
    case TestJsonValue::Type::Number:
      ss << v.number_val;
      break;
    case TestJsonValue::Type::String:
      ss << "\"" << escape_json_string(v.string_val) << "\"";
      break;
    case TestJsonValue::Type::List:
      ss << "[";
      for (size_t i = 0; i < v.list_val.size(); i++) {
        if (i > 0) {
          ss << ",";
        }
        ss << json_value_to_string(v.list_val[i]);
      }
      ss << "]";
      break;
    case TestJsonValue::Type::Object:
      ss << "{";
      for (size_t i = 0; i < v.object_val.size(); i++) {
        if (i > 0) {
          ss << ",";
        }
        ss << "\"" << v.object_val[i].first
           << "\":" << json_value_to_string(v.object_val[i].second);
      }
      ss << "}";
      break;
  }
  return ss.str();
}

static std::string uuid_to_hex(const TestUUID& id)
{
  std::ostringstream ss;
  for (int i = 0; i < 16; i++) {
    ss << std::hex << std::setfill('0') << std::setw(2) << (int)id.bytes[i];
  }
  return ss.str();
}

static std::string person_to_json(const TestPerson& p)
{
  std::ostringstream ss;
  ss << "{\"id\":" << p.id << ",\"name\":\"" << p.name << "\",\"email\":\"" << p.email
     << "\",\"age\":" << p.age << "}";
  return ss.str();
}

static std::string order_to_json(const TestOrder& o)
{
  std::ostringstream ss;
  ss << "{\"order_id\":" << o.order_id << ",\"customer_id\":" << o.customer_id << ",\"item_ids\":[";
  for (size_t i = 0; i < o.item_ids.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << o.item_ids[i];
  }
  ss << "],\"quantities\":[";
  for (size_t i = 0; i < o.quantities.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << o.quantities[i];
  }
  ss << "],\"total\":" << o.total << ",\"timestamp\":" << o.timestamp << "}";
  return ss.str();
}

static std::string event_to_json(const TestEvent& e)
{
  std::ostringstream ss;
  ss << "{\"id\":" << e.id << ",\"type\":\"" << e.type << "\",\"source\":\"" << e.source
     << "\",\"timestamp\":" << e.timestamp << ",\"payload_size\":" << e.payload.size() << "}";
  return ss.str();
}

static std::string tree_to_json(const TestTreeNode& t)
{
  std::ostringstream ss;
  ss << "{\"value\":" << t.value << ",\"children\":[";
  for (size_t i = 0; i < t.children.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << tree_to_json(t.children[i]);
  }
  ss << "]}";
  return ss.str();
}

static std::string document_to_json(const TestDocument& d)
{
  std::ostringstream ss;
  ss << "{\"title\":\"" << d.title << "\",\"body\":\"" << escape_json_string(d.body)
     << "\",\"metadata\":{";
  for (size_t i = 0; i < d.metadata.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << "\"" << d.metadata[i].first << "\":" << json_value_to_string(d.metadata[i].second);
  }
  ss << "}}";
  return ss.str();
}

static std::string chunked_text_to_json(const TestChunkedText& c)
{
  std::ostringstream ss;
  ss << "{\"source\":\"" << escape_json_string(c.source) << "\",\"spans\":[";
  for (size_t i = 0; i < c.spans.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << "{\"start\":" << c.spans[i].start << ",\"len\":" << c.spans[i].len
       << ",\"kind\":" << (int)c.spans[i].kind << "}";
  }
  ss << "]}";
  return ss.str();
}

static std::string embedding_bf16_to_json(const TestEmbeddingBF16& e)
{
  std::ostringstream ss;
  ss << "{\"id\":\"" << uuid_to_hex(e.id) << "\",\"vector\":[";
  for (size_t i = 0; i < e.vector.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << e.vector[i];
  }
  ss << "]}";
  return ss.str();
}

static std::string embedding_batch_to_json(const TestEmbeddingBatch& b)
{
  std::ostringstream ss;
  ss << "{\"model\":\"" << b.model << "\",\"embeddings\":[";
  for (size_t i = 0; i < b.embeddings.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << embedding_bf16_to_json(b.embeddings[i]);
  }
  ss << "],\"usage_tokens\":" << b.usage_tokens << "}";
  return ss.str();
}

static std::string llm_chunk_to_json(const TestLLMStreamChunk& c)
{
  std::ostringstream ss;
  ss << "{\"chunk_id\":" << c.chunk_id << ",\"tokens\":[";
  for (size_t i = 0; i < c.tokens.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << "\"" << escape_json_string(c.tokens[i]) << "\"";
  }
  ss << "],\"logprobs\":[";
  for (size_t i = 0; i < c.logprobs.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << "{\"top_tokens\":[";
    for (size_t j = 0; j < c.logprobs[i].top_tokens.size(); j++) {
      if (j > 0) {
        ss << ",";
      }
      const auto& t = c.logprobs[i].top_tokens[j];
      ss << "{\"token\":\"" << escape_json_string(t.token) << "\",\"token_id\":" << t.token_id
         << ",\"logprob\":" << t.logprob << "}";
    }
    ss << "]}";
  }
  ss << "],\"finish_reason\":\"" << c.finish_reason << "\"}";
  return ss.str();
}

static std::string tensor_shard_to_json(const TestTensorShard& t)
{
  std::ostringstream ss;
  ss << "{\"name\":\"" << t.name << "\",\"shape\":[";
  for (size_t i = 0; i < t.shape.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << t.shape[i];
  }
  ss << "],\"dtype\":\"" << t.dtype << "\",\"data\":[";
  for (size_t i = 0; i < t.data.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << t.data[i];
  }
  ss << "],\"offset\":" << t.offset << ",\"total_elements\":" << t.total_elements << "}";
  return ss.str();
}

static std::string inference_response_to_json(const TestInferenceResponse& r)
{
  std::ostringstream ss;
  ss << "{\"request_id\":\"" << uuid_to_hex(r.request_id) << "\",\"embeddings\":[";
  for (size_t i = 0; i < r.embeddings.size(); i++) {
    if (i > 0) {
      ss << ",";
    }
    ss << embedding_bf16_to_json(r.embeddings[i]);
  }
  ss << "],\"timing\":{\"queue_time\":{\"seconds\":" << r.timing.queue_time.seconds
     << ",\"nanos\":" << r.timing.queue_time.nanos
     << "},\"inference_time\":{\"seconds\":" << r.timing.inference_time.seconds
     << ",\"nanos\":" << r.timing.inference_time.nanos
     << "},\"tokens_per_second\":" << r.timing.tokens_per_second << "}}";
  return ss.str();
}

// Pre-generated JSON strings
static std::string g_json_person_small;
static std::string g_json_person_medium;
static std::string g_json_order_small;
static std::string g_json_order_large;
static std::string g_json_event_small;
static std::string g_json_event_large;
static std::string g_json_tree_wide;
static std::string g_json_tree_deep;
static std::string g_json_small;
static std::string g_json_large;
static std::string g_json_doc_small;
static std::string g_json_doc_large;
static std::string g_json_chunked_text;
static std::string g_json_embedding_768;
static std::string g_json_embedding_1536;
static std::string g_json_embedding_batch;
static std::string g_json_llm_chunk_large;
static std::string g_json_tensor_small;
static std::string g_json_tensor_large;
static std::string g_json_inference;
static bool g_json_initialized = false;

static void ensure_json()
{
  if (g_json_initialized) {
    return;
  }
  g_json_person_small = person_to_json(GetSmallPerson());
  g_json_person_medium = person_to_json(GetMediumPerson());
  g_json_order_small = order_to_json(GetSmallOrder());
  g_json_order_large = order_to_json(GetLargeOrder());
  g_json_event_small = event_to_json(GetSmallEvent());
  g_json_event_large = event_to_json(GetLargeEvent());
  g_json_tree_wide = tree_to_json(GetWideTree());
  g_json_tree_deep = tree_to_json(GetDeepTree());
  g_json_small = json_value_to_string(GetSmallJson());
  g_json_large = json_value_to_string(GetLargeJson());
  g_json_doc_small = document_to_json(GetSmallDocument());
  g_json_doc_large = document_to_json(GetLargeDocument());
  g_json_chunked_text = chunked_text_to_json(GetAliceChunks());
  g_json_embedding_768 = embedding_bf16_to_json(GetEmbedding768());
  g_json_embedding_1536 = embedding_bf16_to_json(GetEmbedding1536());
  g_json_embedding_batch = embedding_batch_to_json(GetEmbeddingBatch());
  g_json_llm_chunk_large = llm_chunk_to_json(GetLLMChunkLarge());
  g_json_tensor_small = tensor_shard_to_json(GetTensorShardSmall());
  g_json_tensor_large = tensor_shard_to_json(GetTensorShardLarge());
  g_json_inference = inference_response_to_json(GetInferenceResponse());
  g_json_initialized = true;
}

static void BM_Simdjson_Parse_PersonSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_person_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_person_small.size());
}

static void BM_Simdjson_Parse_PersonMedium(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_person_medium);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_person_medium.size());
}

static void BM_Simdjson_Parse_OrderSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_order_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["order_id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_order_small.size());
}

static void BM_Simdjson_Parse_OrderLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_order_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["order_id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_order_large.size());
}

static void BM_Simdjson_Parse_EventSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_event_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_event_small.size());
}

static void BM_Simdjson_Parse_EventLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_event_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_event_large.size());
}

static void BM_Simdjson_Parse_TreeWide(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_tree_wide);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t val = doc["value"].get_int64();
    benchmark::DoNotOptimize(val);
  }
  state.SetBytesProcessed(state.iterations() * g_json_tree_wide.size());
}

static void BM_Simdjson_Parse_TreeDeep(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_tree_deep);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t val = doc["value"].get_int64();
    benchmark::DoNotOptimize(val);
  }
  state.SetBytesProcessed(state.iterations() * g_json_tree_deep.size());
}

static void BM_Simdjson_Parse_JsonSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    auto type = doc.type();
    benchmark::DoNotOptimize(type);
  }
  state.SetBytesProcessed(state.iterations() * g_json_small.size());
}

static void BM_Simdjson_Parse_JsonLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    auto type = doc.type();
    benchmark::DoNotOptimize(type);
  }
  state.SetBytesProcessed(state.iterations() * g_json_large.size());
}

static void BM_Simdjson_Parse_DocumentSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_doc_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view title = doc["title"].get_string();
    benchmark::DoNotOptimize(title);
  }
  state.SetBytesProcessed(state.iterations() * g_json_doc_small.size());
}

static void BM_Simdjson_Parse_DocumentLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_doc_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view title = doc["title"].get_string();
    benchmark::DoNotOptimize(title);
  }
  state.SetBytesProcessed(state.iterations() * g_json_doc_large.size());
}

static void BM_Simdjson_Parse_ChunkedText(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_chunked_text);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view source = doc["source"].get_string();
    benchmark::DoNotOptimize(source);
  }
  state.SetBytesProcessed(state.iterations() * g_json_chunked_text.size());
}

static void BM_Simdjson_Parse_Embedding768(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_embedding_768);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    auto values = doc["vector"].get_array();
    size_t count = 0;
    for (auto v : values) {
      count++;
      benchmark::DoNotOptimize(v);
    }
    benchmark::DoNotOptimize(count);
  }
  state.SetBytesProcessed(state.iterations() * g_json_embedding_768.size());
}

static void BM_Simdjson_Parse_Embedding1536(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_embedding_1536);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    auto values = doc["vector"].get_array();
    size_t count = 0;
    for (auto v : values) {
      count++;
      benchmark::DoNotOptimize(v);
    }
    benchmark::DoNotOptimize(count);
  }
  state.SetBytesProcessed(state.iterations() * g_json_embedding_1536.size());
}

static void BM_Simdjson_Parse_EmbeddingBatch(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_embedding_batch);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    auto embeddings = doc["embeddings"].get_array();
    size_t count = 0;
    for (auto e : embeddings) {
      count++;
      benchmark::DoNotOptimize(e);
    }
    benchmark::DoNotOptimize(count);
  }
  state.SetBytesProcessed(state.iterations() * g_json_embedding_batch.size());
}

static void BM_Simdjson_Parse_LLMChunkLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_llm_chunk_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    int64_t id = doc["chunk_id"].get_int64();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_llm_chunk_large.size());
}

static void BM_Simdjson_Parse_TensorShardSmall(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_tensor_small);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view name = doc["name"].get_string();
    benchmark::DoNotOptimize(name);
  }
  state.SetBytesProcessed(state.iterations() * g_json_tensor_small.size());
}

static void BM_Simdjson_Parse_TensorShardLarge(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_tensor_large);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view name = doc["name"].get_string();
    benchmark::DoNotOptimize(name);
  }
  state.SetBytesProcessed(state.iterations() * g_json_tensor_large.size());
}

static void BM_Simdjson_Parse_InferenceResponse(benchmark::State& state)
{
  ensure_json();
  auto padded = simdjson::padded_string(g_json_inference);
  for (auto _ : state) {
    auto doc = g_parser.iterate(padded);
    std::string_view id = doc["request_id"].get_string();
    benchmark::DoNotOptimize(id);
  }
  state.SetBytesProcessed(state.iterations() * g_json_inference.size());
}

void RegisterSimdjsonBenchmarks()
{
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_PersonSmall", BM_Simdjson_Parse_PersonSmall);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_PersonMedium", BM_Simdjson_Parse_PersonMedium);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_OrderSmall", BM_Simdjson_Parse_OrderSmall);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_OrderLarge", BM_Simdjson_Parse_OrderLarge);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_EventSmall", BM_Simdjson_Parse_EventSmall);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_EventLarge", BM_Simdjson_Parse_EventLarge);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_TreeWide", BM_Simdjson_Parse_TreeWide);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_TreeDeep", BM_Simdjson_Parse_TreeDeep);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_JsonSmall", BM_Simdjson_Parse_JsonSmall);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_JsonLarge", BM_Simdjson_Parse_JsonLarge);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_DocumentSmall", BM_Simdjson_Parse_DocumentSmall);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_DocumentLarge", BM_Simdjson_Parse_DocumentLarge);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_ChunkedText", BM_Simdjson_Parse_ChunkedText);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_Embedding768", BM_Simdjson_Parse_Embedding768);
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_Embedding1536", BM_Simdjson_Parse_Embedding1536);
  benchmark::RegisterBenchmark(
      "BM_Simdjson_Parse_EmbeddingBatch", BM_Simdjson_Parse_EmbeddingBatch
  );
  benchmark::RegisterBenchmark("BM_Simdjson_Parse_LLMChunkLarge", BM_Simdjson_Parse_LLMChunkLarge);
  benchmark::RegisterBenchmark(
      "BM_Simdjson_Parse_TensorShardSmall", BM_Simdjson_Parse_TensorShardSmall
  );
  benchmark::RegisterBenchmark(
      "BM_Simdjson_Parse_TensorShardLarge", BM_Simdjson_Parse_TensorShardLarge
  );
  benchmark::RegisterBenchmark(
      "BM_Simdjson_Parse_InferenceResponse", BM_Simdjson_Parse_InferenceResponse
  );
}
