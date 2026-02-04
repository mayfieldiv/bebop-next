#include <benchmark/benchmark.h>
#include <msgpack.h>

#include "bench_harness.h"
#include <cstring>
#include <vector>

static msgpack_sbuffer g_sbuf;
static msgpack_packer g_pk;
static bool g_packer_initialized = false;

static void ensure_packer()
{
  if (!g_packer_initialized) {
    msgpack_sbuffer_init(&g_sbuf);
    msgpack_packer_init(&g_pk, &g_sbuf, msgpack_sbuffer_write);
    g_packer_initialized = true;
  }
}

static std::vector<char> encode_person(const TestPerson& p)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 4);
  msgpack_pack_str(&g_pk, 2);
  msgpack_pack_str_body(&g_pk, "id", 2);
  msgpack_pack_int32(&g_pk, p.id);
  msgpack_pack_str(&g_pk, 4);
  msgpack_pack_str_body(&g_pk, "name", 4);
  msgpack_pack_str(&g_pk, p.name.size());
  msgpack_pack_str_body(&g_pk, p.name.c_str(), p.name.size());
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "email", 5);
  msgpack_pack_str(&g_pk, p.email.size());
  msgpack_pack_str_body(&g_pk, p.email.c_str(), p.email.size());
  msgpack_pack_str(&g_pk, 3);
  msgpack_pack_str_body(&g_pk, "age", 3);
  msgpack_pack_int32(&g_pk, p.age);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_order(const TestOrder& o)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 6);
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "order_id", 8);
  msgpack_pack_int64(&g_pk, o.order_id);
  msgpack_pack_str(&g_pk, 11);
  msgpack_pack_str_body(&g_pk, "customer_id", 11);
  msgpack_pack_int64(&g_pk, o.customer_id);
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "item_ids", 8);
  msgpack_pack_array(&g_pk, o.item_ids.size());
  for (auto id : o.item_ids) {
    msgpack_pack_int64(&g_pk, id);
  }
  msgpack_pack_str(&g_pk, 10);
  msgpack_pack_str_body(&g_pk, "quantities", 10);
  msgpack_pack_array(&g_pk, o.quantities.size());
  for (auto q : o.quantities) {
    msgpack_pack_int32(&g_pk, q);
  }
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "total", 5);
  msgpack_pack_double(&g_pk, o.total);
  msgpack_pack_str(&g_pk, 9);
  msgpack_pack_str_body(&g_pk, "timestamp", 9);
  msgpack_pack_int64(&g_pk, o.timestamp);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_event(const TestEvent& e)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 5);
  msgpack_pack_str(&g_pk, 2);
  msgpack_pack_str_body(&g_pk, "id", 2);
  msgpack_pack_int64(&g_pk, e.id);
  msgpack_pack_str(&g_pk, 4);
  msgpack_pack_str_body(&g_pk, "type", 4);
  msgpack_pack_str(&g_pk, e.type.size());
  msgpack_pack_str_body(&g_pk, e.type.c_str(), e.type.size());
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "source", 6);
  msgpack_pack_str(&g_pk, e.source.size());
  msgpack_pack_str_body(&g_pk, e.source.c_str(), e.source.size());
  msgpack_pack_str(&g_pk, 9);
  msgpack_pack_str_body(&g_pk, "timestamp", 9);
  msgpack_pack_int64(&g_pk, e.timestamp);
  msgpack_pack_str(&g_pk, 7);
  msgpack_pack_str_body(&g_pk, "payload", 7);
  msgpack_pack_bin(&g_pk, e.payload.size());
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.payload.data()), e.payload.size());
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static void encode_tree_recursive(const TestTreeNode& t)
{
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "value", 5);
  msgpack_pack_int32(&g_pk, t.value);
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "children", 8);
  msgpack_pack_array(&g_pk, t.children.size());
  for (const auto& child : t.children) {
    encode_tree_recursive(child);
  }
}

static std::vector<char> encode_tree(const TestTreeNode& t)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  encode_tree_recursive(t);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static void encode_json_value(const TestJsonValue& v);

static void encode_json_value(const TestJsonValue& v)
{
  switch (v.type) {
    case TestJsonValue::Type::Null:
      msgpack_pack_nil(&g_pk);
      break;
    case TestJsonValue::Type::Bool:
      if (v.bool_val) {
        msgpack_pack_true(&g_pk);
      } else {
        msgpack_pack_false(&g_pk);
      }
      break;
    case TestJsonValue::Type::Number:
      msgpack_pack_double(&g_pk, v.number_val);
      break;
    case TestJsonValue::Type::String:
      msgpack_pack_str(&g_pk, v.string_val.size());
      msgpack_pack_str_body(&g_pk, v.string_val.c_str(), v.string_val.size());
      break;
    case TestJsonValue::Type::List:
      msgpack_pack_array(&g_pk, v.list_val.size());
      for (const auto& item : v.list_val) {
        encode_json_value(item);
      }
      break;
    case TestJsonValue::Type::Object:
      msgpack_pack_map(&g_pk, v.object_val.size());
      for (const auto& [key, val] : v.object_val) {
        msgpack_pack_str(&g_pk, key.size());
        msgpack_pack_str_body(&g_pk, key.c_str(), key.size());
        encode_json_value(val);
      }
      break;
  }
}

static std::vector<char> encode_json(const TestJsonValue& v)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  encode_json_value(v);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_document(const TestDocument& d)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 3);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "title", 5);
  msgpack_pack_str(&g_pk, d.title.size());
  msgpack_pack_str_body(&g_pk, d.title.c_str(), d.title.size());
  msgpack_pack_str(&g_pk, 4);
  msgpack_pack_str_body(&g_pk, "body", 4);
  msgpack_pack_str(&g_pk, d.body.size());
  msgpack_pack_str_body(&g_pk, d.body.c_str(), d.body.size());
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "metadata", 8);
  msgpack_pack_map(&g_pk, d.metadata.size());
  for (const auto& [key, val] : d.metadata) {
    msgpack_pack_str(&g_pk, key.size());
    msgpack_pack_str_body(&g_pk, key.c_str(), key.size());
    encode_json_value(val);
  }
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_chunked_text(const TestChunkedText& c)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "source", 6);
  msgpack_pack_str(&g_pk, c.source.size());
  msgpack_pack_str_body(&g_pk, c.source.c_str(), c.source.size());
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "spans", 5);
  msgpack_pack_array(&g_pk, c.spans.size());
  for (const auto& s : c.spans) {
    msgpack_pack_array(&g_pk, 3);
    msgpack_pack_uint32(&g_pk, s.start);
    msgpack_pack_uint32(&g_pk, s.len);
    msgpack_pack_uint8(&g_pk, static_cast<uint8_t>(s.kind));
  }
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static void encode_embedding_bf16(const TestEmbeddingBF16& e)
{
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 2);
  msgpack_pack_str_body(&g_pk, "id", 2);
  msgpack_pack_bin(&g_pk, 16);
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.id.bytes), 16);
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "vector", 6);
  msgpack_pack_bin(&g_pk, e.vector.size() * 2);
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.vector.data()), e.vector.size() * 2);
}

static std::vector<char> encode_embedding_bf16_vec(const TestEmbeddingBF16& e)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  encode_embedding_bf16(e);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_embedding_f32(const TestEmbeddingF32& e)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 2);
  msgpack_pack_str_body(&g_pk, "id", 2);
  msgpack_pack_bin(&g_pk, 16);
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.id.bytes), 16);
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "vector", 6);
  msgpack_pack_array(&g_pk, e.vector.size());
  for (float f : e.vector) {
    msgpack_pack_float(&g_pk, f);
  }
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_embedding_batch(const TestEmbeddingBatch& b)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 3);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "model", 5);
  msgpack_pack_str(&g_pk, b.model.size());
  msgpack_pack_str_body(&g_pk, b.model.c_str(), b.model.size());
  msgpack_pack_str(&g_pk, 10);
  msgpack_pack_str_body(&g_pk, "embeddings", 10);
  msgpack_pack_array(&g_pk, b.embeddings.size());
  for (const auto& e : b.embeddings) {
    encode_embedding_bf16(e);
  }
  msgpack_pack_str(&g_pk, 12);
  msgpack_pack_str_body(&g_pk, "usage_tokens", 12);
  msgpack_pack_uint32(&g_pk, b.usage_tokens);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static void encode_token_logprob(const TestTokenLogprob& lp)
{
  msgpack_pack_map(&g_pk, 3);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "token", 5);
  msgpack_pack_str(&g_pk, lp.token.size());
  msgpack_pack_str_body(&g_pk, lp.token.c_str(), lp.token.size());
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "token_id", 8);
  msgpack_pack_uint32(&g_pk, lp.token_id);
  msgpack_pack_str(&g_pk, 7);
  msgpack_pack_str_body(&g_pk, "logprob", 7);
  msgpack_pack_float(&g_pk, lp.logprob);
}

static std::vector<char> encode_llm_chunk(const TestLLMStreamChunk& c)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 4);
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "chunk_id", 8);
  msgpack_pack_uint32(&g_pk, c.chunk_id);
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "tokens", 6);
  msgpack_pack_array(&g_pk, c.tokens.size());
  for (const auto& t : c.tokens) {
    msgpack_pack_str(&g_pk, t.size());
    msgpack_pack_str_body(&g_pk, t.c_str(), t.size());
  }
  msgpack_pack_str(&g_pk, 8);
  msgpack_pack_str_body(&g_pk, "logprobs", 8);
  msgpack_pack_array(&g_pk, c.logprobs.size());
  for (const auto& alt : c.logprobs) {
    msgpack_pack_map(&g_pk, 1);
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "top_tokens", 10);
    msgpack_pack_array(&g_pk, alt.top_tokens.size());
    for (const auto& lp : alt.top_tokens) {
      encode_token_logprob(lp);
    }
  }
  msgpack_pack_str(&g_pk, 13);
  msgpack_pack_str_body(&g_pk, "finish_reason", 13);
  msgpack_pack_str(&g_pk, c.finish_reason.size());
  msgpack_pack_str_body(&g_pk, c.finish_reason.c_str(), c.finish_reason.size());
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_tensor_shard(const TestTensorShard& t)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 6);
  msgpack_pack_str(&g_pk, 4);
  msgpack_pack_str_body(&g_pk, "name", 4);
  msgpack_pack_str(&g_pk, t.name.size());
  msgpack_pack_str_body(&g_pk, t.name.c_str(), t.name.size());
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "shape", 5);
  msgpack_pack_array(&g_pk, t.shape.size());
  for (uint32_t s : t.shape) {
    msgpack_pack_uint32(&g_pk, s);
  }
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "dtype", 5);
  msgpack_pack_str(&g_pk, t.dtype.size());
  msgpack_pack_str_body(&g_pk, t.dtype.c_str(), t.dtype.size());
  msgpack_pack_str(&g_pk, 4);
  msgpack_pack_str_body(&g_pk, "data", 4);
  msgpack_pack_bin(&g_pk, t.data.size() * 2);
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(t.data.data()), t.data.size() * 2);
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "offset", 6);
  msgpack_pack_uint64(&g_pk, t.offset);
  msgpack_pack_str(&g_pk, 14);
  msgpack_pack_str_body(&g_pk, "total_elements", 14);
  msgpack_pack_uint64(&g_pk, t.total_elements);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> encode_inference_response(const TestInferenceResponse& r)
{
  ensure_packer();
  msgpack_sbuffer_clear(&g_sbuf);
  msgpack_pack_map(&g_pk, 3);
  msgpack_pack_str(&g_pk, 10);
  msgpack_pack_str_body(&g_pk, "request_id", 10);
  msgpack_pack_bin(&g_pk, 16);
  msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(r.request_id.bytes), 16);
  msgpack_pack_str(&g_pk, 10);
  msgpack_pack_str_body(&g_pk, "embeddings", 10);
  msgpack_pack_array(&g_pk, r.embeddings.size());
  for (const auto& e : r.embeddings) {
    encode_embedding_bf16(e);
  }
  msgpack_pack_str(&g_pk, 6);
  msgpack_pack_str_body(&g_pk, "timing", 6);
  msgpack_pack_map(&g_pk, 3);
  msgpack_pack_str(&g_pk, 10);
  msgpack_pack_str_body(&g_pk, "queue_time", 10);
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 7);
  msgpack_pack_str_body(&g_pk, "seconds", 7);
  msgpack_pack_int64(&g_pk, r.timing.queue_time.seconds);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "nanos", 5);
  msgpack_pack_int32(&g_pk, r.timing.queue_time.nanos);
  msgpack_pack_str(&g_pk, 14);
  msgpack_pack_str_body(&g_pk, "inference_time", 14);
  msgpack_pack_map(&g_pk, 2);
  msgpack_pack_str(&g_pk, 7);
  msgpack_pack_str_body(&g_pk, "seconds", 7);
  msgpack_pack_int64(&g_pk, r.timing.inference_time.seconds);
  msgpack_pack_str(&g_pk, 5);
  msgpack_pack_str_body(&g_pk, "nanos", 5);
  msgpack_pack_int32(&g_pk, r.timing.inference_time.nanos);
  msgpack_pack_str(&g_pk, 17);
  msgpack_pack_str_body(&g_pk, "tokens_per_second", 17);
  msgpack_pack_float(&g_pk, r.timing.tokens_per_second);
  return std::vector<char>(g_sbuf.data, g_sbuf.data + g_sbuf.size);
}

static std::vector<char> g_encoded_person_small;
static std::vector<char> g_encoded_person_medium;
static std::vector<char> g_encoded_order_small;
static std::vector<char> g_encoded_order_large;
static std::vector<char> g_encoded_event_small;
static std::vector<char> g_encoded_event_large;
static std::vector<char> g_encoded_tree_wide;
static std::vector<char> g_encoded_tree_deep;
static std::vector<char> g_encoded_json_small;
static std::vector<char> g_encoded_json_large;
static std::vector<char> g_encoded_doc_small;
static std::vector<char> g_encoded_doc_large;
static std::vector<char> g_encoded_alice;
static std::vector<char> g_encoded_emb_768;
static std::vector<char> g_encoded_emb_1536;
static std::vector<char> g_encoded_emb_f32_768;
static std::vector<char> g_encoded_emb_batch;
static std::vector<char> g_encoded_llm_small;
static std::vector<char> g_encoded_llm_large;
static std::vector<char> g_encoded_tensor_small;
static std::vector<char> g_encoded_tensor_large;
static std::vector<char> g_encoded_inference;
static bool g_initialized = false;

static void init_msgpack_data()
{
  if (g_initialized) {
    return;
  }
  g_encoded_person_small = encode_person(GetSmallPerson());
  g_encoded_person_medium = encode_person(GetMediumPerson());
  g_encoded_order_small = encode_order(GetSmallOrder());
  g_encoded_order_large = encode_order(GetLargeOrder());
  g_encoded_event_small = encode_event(GetSmallEvent());
  g_encoded_event_large = encode_event(GetLargeEvent());
  g_encoded_tree_wide = encode_tree(GetWideTree());
  g_encoded_tree_deep = encode_tree(GetDeepTree());
  g_encoded_json_small = encode_json(GetSmallJson());
  g_encoded_json_large = encode_json(GetLargeJson());
  g_encoded_doc_small = encode_document(GetSmallDocument());
  g_encoded_doc_large = encode_document(GetLargeDocument());
  g_encoded_alice = encode_chunked_text(GetAliceChunks());
  g_encoded_emb_768 = encode_embedding_bf16_vec(GetEmbedding768());
  g_encoded_emb_1536 = encode_embedding_bf16_vec(GetEmbedding1536());
  g_encoded_emb_f32_768 = encode_embedding_f32(GetEmbeddingF32_768());
  g_encoded_emb_batch = encode_embedding_batch(GetEmbeddingBatch());
  g_encoded_llm_small = encode_llm_chunk(GetLLMChunkSmall());
  g_encoded_llm_large = encode_llm_chunk(GetLLMChunkLarge());
  g_encoded_tensor_small = encode_tensor_shard(GetTensorShardSmall());
  g_encoded_tensor_large = encode_tensor_shard(GetTensorShardLarge());
  g_encoded_inference = encode_inference_response(GetInferenceResponse());
  g_initialized = true;
}

static void BM_Msgpack_Encode_PersonSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& p = GetSmallPerson();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 4);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int32(&g_pk, p.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "name", 4);
    msgpack_pack_str(&g_pk, p.name.size());
    msgpack_pack_str_body(&g_pk, p.name.c_str(), p.name.size());
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "email", 5);
    msgpack_pack_str(&g_pk, p.email.size());
    msgpack_pack_str_body(&g_pk, p.email.c_str(), p.email.size());
    msgpack_pack_str(&g_pk, 3);
    msgpack_pack_str_body(&g_pk, "age", 3);
    msgpack_pack_int32(&g_pk, p.age);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_small.size());
}

static void BM_Msgpack_Encode_PersonMedium(benchmark::State& state)
{
  init_msgpack_data();
  const auto& p = GetMediumPerson();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 4);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int32(&g_pk, p.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "name", 4);
    msgpack_pack_str(&g_pk, p.name.size());
    msgpack_pack_str_body(&g_pk, p.name.c_str(), p.name.size());
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "email", 5);
    msgpack_pack_str(&g_pk, p.email.size());
    msgpack_pack_str_body(&g_pk, p.email.c_str(), p.email.size());
    msgpack_pack_str(&g_pk, 3);
    msgpack_pack_str_body(&g_pk, "age", 3);
    msgpack_pack_int32(&g_pk, p.age);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_medium.size());
}

static void BM_Msgpack_Encode_OrderSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& o = GetSmallOrder();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 6);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "order_id", 8);
    msgpack_pack_int64(&g_pk, o.order_id);
    msgpack_pack_str(&g_pk, 11);
    msgpack_pack_str_body(&g_pk, "customer_id", 11);
    msgpack_pack_int64(&g_pk, o.customer_id);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "item_ids", 8);
    msgpack_pack_array(&g_pk, o.item_ids.size());
    for (auto id : o.item_ids) {
      msgpack_pack_int64(&g_pk, id);
    }
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "quantities", 10);
    msgpack_pack_array(&g_pk, o.quantities.size());
    for (auto q : o.quantities) {
      msgpack_pack_int32(&g_pk, q);
    }
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "total", 5);
    msgpack_pack_double(&g_pk, o.total);
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, o.timestamp);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_small.size());
}

static void BM_Msgpack_Encode_OrderLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& o = GetLargeOrder();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 6);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "order_id", 8);
    msgpack_pack_int64(&g_pk, o.order_id);
    msgpack_pack_str(&g_pk, 11);
    msgpack_pack_str_body(&g_pk, "customer_id", 11);
    msgpack_pack_int64(&g_pk, o.customer_id);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "item_ids", 8);
    msgpack_pack_array(&g_pk, o.item_ids.size());
    for (auto id : o.item_ids) {
      msgpack_pack_int64(&g_pk, id);
    }
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "quantities", 10);
    msgpack_pack_array(&g_pk, o.quantities.size());
    for (auto q : o.quantities) {
      msgpack_pack_int32(&g_pk, q);
    }
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "total", 5);
    msgpack_pack_double(&g_pk, o.total);
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, o.timestamp);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_large.size());
}

static void BM_Msgpack_Encode_EventSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& e = GetSmallEvent();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 5);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int64(&g_pk, e.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "type", 4);
    msgpack_pack_str(&g_pk, e.type.size());
    msgpack_pack_str_body(&g_pk, e.type.c_str(), e.type.size());
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "source", 6);
    msgpack_pack_str(&g_pk, e.source.size());
    msgpack_pack_str_body(&g_pk, e.source.c_str(), e.source.size());
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, e.timestamp);
    msgpack_pack_str(&g_pk, 7);
    msgpack_pack_str_body(&g_pk, "payload", 7);
    msgpack_pack_bin(&g_pk, e.payload.size());
    msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.payload.data()), e.payload.size());
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_small.size());
}

static void BM_Msgpack_Encode_EventLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& e = GetLargeEvent();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 5);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int64(&g_pk, e.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "type", 4);
    msgpack_pack_str(&g_pk, e.type.size());
    msgpack_pack_str_body(&g_pk, e.type.c_str(), e.type.size());
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "source", 6);
    msgpack_pack_str(&g_pk, e.source.size());
    msgpack_pack_str_body(&g_pk, e.source.c_str(), e.source.size());
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, e.timestamp);
    msgpack_pack_str(&g_pk, 7);
    msgpack_pack_str_body(&g_pk, "payload", 7);
    msgpack_pack_bin(&g_pk, e.payload.size());
    msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.payload.data()), e.payload.size());
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_large.size());
}

static void BM_Msgpack_Encode_TreeWide(benchmark::State& state)
{
  init_msgpack_data();
  const auto& t = GetWideTree();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_tree_recursive(t);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_wide.size());
}

static void BM_Msgpack_Encode_TreeDeep(benchmark::State& state)
{
  init_msgpack_data();
  const auto& t = GetDeepTree();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_tree_recursive(t);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_deep.size());
}

static void BM_Msgpack_Encode_JsonSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& v = GetSmallJson();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_json_value(v);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_small.size());
}

static void BM_Msgpack_Encode_JsonLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& v = GetLargeJson();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_json_value(v);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_large.size());
}

static void BM_Msgpack_Encode_DocumentSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& d = GetSmallDocument();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 3);
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "title", 5);
    msgpack_pack_str(&g_pk, d.title.size());
    msgpack_pack_str_body(&g_pk, d.title.c_str(), d.title.size());
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "body", 4);
    msgpack_pack_str(&g_pk, d.body.size());
    msgpack_pack_str_body(&g_pk, d.body.c_str(), d.body.size());
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "metadata", 8);
    msgpack_pack_map(&g_pk, d.metadata.size());
    for (const auto& [key, val] : d.metadata) {
      msgpack_pack_str(&g_pk, key.size());
      msgpack_pack_str_body(&g_pk, key.c_str(), key.size());
      encode_json_value(val);
    }
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_small.size());
}

static void BM_Msgpack_Encode_DocumentLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& d = GetLargeDocument();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 3);
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "title", 5);
    msgpack_pack_str(&g_pk, d.title.size());
    msgpack_pack_str_body(&g_pk, d.title.c_str(), d.title.size());
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "body", 4);
    msgpack_pack_str(&g_pk, d.body.size());
    msgpack_pack_str_body(&g_pk, d.body.c_str(), d.body.size());
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "metadata", 8);
    msgpack_pack_map(&g_pk, d.metadata.size());
    for (const auto& [key, val] : d.metadata) {
      msgpack_pack_str(&g_pk, key.size());
      msgpack_pack_str_body(&g_pk, key.c_str(), key.size());
      encode_json_value(val);
    }
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_large.size());
}

static void BM_Msgpack_Encode_ChunkedText(benchmark::State& state)
{
  init_msgpack_data();
  const auto& c = GetAliceChunks();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 2);
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "source", 6);
    msgpack_pack_str(&g_pk, c.source.size());
    msgpack_pack_str_body(&g_pk, c.source.c_str(), c.source.size());
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "spans", 5);
    msgpack_pack_array(&g_pk, c.spans.size());
    for (const auto& s : c.spans) {
      msgpack_pack_array(&g_pk, 3);
      msgpack_pack_uint32(&g_pk, s.start);
      msgpack_pack_uint32(&g_pk, s.len);
      msgpack_pack_uint8(&g_pk, static_cast<uint8_t>(s.kind));
    }
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_alice.size());
}

static void BM_Msgpack_Decode_PersonSmall(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_person_small.data(), g_encoded_person_small.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_small.size());
}

static void BM_Msgpack_Decode_PersonMedium(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_person_medium.data(), g_encoded_person_medium.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_medium.size());
}

static void BM_Msgpack_Decode_OrderSmall(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_order_small.data(), g_encoded_order_small.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_small.size());
}

static void BM_Msgpack_Decode_OrderLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_order_large.data(), g_encoded_order_large.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_large.size());
}

static void BM_Msgpack_Decode_EventSmall(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_event_small.data(), g_encoded_event_small.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_small.size());
}

static void BM_Msgpack_Decode_EventLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_event_large.data(), g_encoded_event_large.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_large.size());
}

static void BM_Msgpack_Decode_TreeWide(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_tree_wide.data(), g_encoded_tree_wide.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_wide.size());
}

static void BM_Msgpack_Decode_TreeDeep(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_tree_deep.data(), g_encoded_tree_deep.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_deep.size());
}

static void BM_Msgpack_Decode_JsonSmall(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_json_small.data(), g_encoded_json_small.size(), nullptr);
    benchmark::DoNotOptimize(result.data.type);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_small.size());
}

static void BM_Msgpack_Decode_JsonLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_json_large.data(), g_encoded_json_large.size(), nullptr);
    benchmark::DoNotOptimize(result.data.type);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_large.size());
}

static void BM_Msgpack_Decode_DocumentSmall(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_doc_small.data(), g_encoded_doc_small.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_small.size());
}

static void BM_Msgpack_Decode_DocumentLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_doc_large.data(), g_encoded_doc_large.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_large.size());
}

static void BM_Msgpack_Decode_ChunkedText(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_alice.data(), g_encoded_alice.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_alice.size());
}

static void BM_Msgpack_Encode_Embedding768(benchmark::State& state)
{
  init_msgpack_data();
  const auto& e = GetEmbedding768();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_embedding_bf16(e);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_768.size());
}

static void BM_Msgpack_Encode_Embedding1536(benchmark::State& state)
{
  init_msgpack_data();
  const auto& e = GetEmbedding1536();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_embedding_bf16(e);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_1536.size());
}

static void BM_Msgpack_Encode_EmbeddingBatch(benchmark::State& state)
{
  init_msgpack_data();
  const auto& b = GetEmbeddingBatch();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 3);
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "model", 5);
    msgpack_pack_str(&g_pk, b.model.size());
    msgpack_pack_str_body(&g_pk, b.model.c_str(), b.model.size());
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "embeddings", 10);
    msgpack_pack_array(&g_pk, b.embeddings.size());
    for (const auto& e : b.embeddings) {
      encode_embedding_bf16(e);
    }
    msgpack_pack_str(&g_pk, 12);
    msgpack_pack_str_body(&g_pk, "usage_tokens", 12);
    msgpack_pack_uint32(&g_pk, b.usage_tokens);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_batch.size());
}

static void BM_Msgpack_Encode_LLMChunkLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& c = GetLLMChunkLarge();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 4);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "chunk_id", 8);
    msgpack_pack_uint32(&g_pk, c.chunk_id);
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "tokens", 6);
    msgpack_pack_array(&g_pk, c.tokens.size());
    for (const auto& t : c.tokens) {
      msgpack_pack_str(&g_pk, t.size());
      msgpack_pack_str_body(&g_pk, t.c_str(), t.size());
    }
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "logprobs", 8);
    msgpack_pack_array(&g_pk, c.logprobs.size());
    for (const auto& alt : c.logprobs) {
      msgpack_pack_map(&g_pk, 1);
      msgpack_pack_str(&g_pk, 10);
      msgpack_pack_str_body(&g_pk, "top_tokens", 10);
      msgpack_pack_array(&g_pk, alt.top_tokens.size());
      for (const auto& lp : alt.top_tokens) {
        encode_token_logprob(lp);
      }
    }
    msgpack_pack_str(&g_pk, 13);
    msgpack_pack_str_body(&g_pk, "finish_reason", 13);
    msgpack_pack_str(&g_pk, c.finish_reason.size());
    msgpack_pack_str_body(&g_pk, c.finish_reason.c_str(), c.finish_reason.size());
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_llm_large.size());
}

static void BM_Msgpack_Encode_TensorShardLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& t = GetTensorShardLarge();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 6);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "name", 4);
    msgpack_pack_str(&g_pk, t.name.size());
    msgpack_pack_str_body(&g_pk, t.name.c_str(), t.name.size());
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "shape", 5);
    msgpack_pack_array(&g_pk, t.shape.size());
    for (uint32_t s : t.shape) {
      msgpack_pack_uint32(&g_pk, s);
    }
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "dtype", 5);
    msgpack_pack_str(&g_pk, t.dtype.size());
    msgpack_pack_str_body(&g_pk, t.dtype.c_str(), t.dtype.size());
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "data", 4);
    msgpack_pack_bin(&g_pk, t.data.size() * 2);
    msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(t.data.data()), t.data.size() * 2);
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "offset", 6);
    msgpack_pack_uint64(&g_pk, t.offset);
    msgpack_pack_str(&g_pk, 14);
    msgpack_pack_str_body(&g_pk, "total_elements", 14);
    msgpack_pack_uint64(&g_pk, t.total_elements);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tensor_large.size());
}

static void BM_Msgpack_Encode_InferenceResponse(benchmark::State& state)
{
  init_msgpack_data();
  const auto& r = GetInferenceResponse();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 3);
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "request_id", 10);
    msgpack_pack_bin(&g_pk, 16);
    msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(r.request_id.bytes), 16);
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "embeddings", 10);
    msgpack_pack_array(&g_pk, r.embeddings.size());
    for (const auto& e : r.embeddings) {
      encode_embedding_bf16(e);
    }
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "timing", 6);
    msgpack_pack_map(&g_pk, 3);
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "queue_time", 10);
    msgpack_pack_map(&g_pk, 2);
    msgpack_pack_str(&g_pk, 7);
    msgpack_pack_str_body(&g_pk, "seconds", 7);
    msgpack_pack_int64(&g_pk, r.timing.queue_time.seconds);
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "nanos", 5);
    msgpack_pack_int32(&g_pk, r.timing.queue_time.nanos);
    msgpack_pack_str(&g_pk, 14);
    msgpack_pack_str_body(&g_pk, "inference_time", 14);
    msgpack_pack_map(&g_pk, 2);
    msgpack_pack_str(&g_pk, 7);
    msgpack_pack_str_body(&g_pk, "seconds", 7);
    msgpack_pack_int64(&g_pk, r.timing.inference_time.seconds);
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "nanos", 5);
    msgpack_pack_int32(&g_pk, r.timing.inference_time.nanos);
    msgpack_pack_str(&g_pk, 17);
    msgpack_pack_str_body(&g_pk, "tokens_per_second", 17);
    msgpack_pack_float(&g_pk, r.timing.tokens_per_second);
    benchmark::DoNotOptimize(g_sbuf.size);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_inference.size());
}

static void BM_Msgpack_Decode_Embedding768(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_emb_768.data(), g_encoded_emb_768.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_768.size());
}

static void BM_Msgpack_Decode_Embedding1536(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_emb_1536.data(), g_encoded_emb_1536.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_1536.size());
}

static void BM_Msgpack_Decode_EmbeddingBatch(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_emb_batch.data(), g_encoded_emb_batch.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_batch.size());
}

static void BM_Msgpack_Decode_LLMChunkLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_llm_large.data(), g_encoded_llm_large.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_llm_large.size());
}

static void BM_Msgpack_Decode_TensorShardLarge(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(
        &result, g_encoded_tensor_large.data(), g_encoded_tensor_large.size(), nullptr
    );
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tensor_large.size());
}

static void BM_Msgpack_Decode_InferenceResponse(benchmark::State& state)
{
  init_msgpack_data();
  for (auto _ : state) {
    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_encoded_inference.data(), g_encoded_inference.size(), nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_inference.size());
}

static void BM_Msgpack_Roundtrip_PersonSmall(benchmark::State& state)
{
  init_msgpack_data();
  const auto& p = GetSmallPerson();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 4);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int32(&g_pk, p.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "name", 4);
    msgpack_pack_str(&g_pk, p.name.size());
    msgpack_pack_str_body(&g_pk, p.name.c_str(), p.name.size());
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "email", 5);
    msgpack_pack_str(&g_pk, p.email.size());
    msgpack_pack_str_body(&g_pk, p.email.c_str(), p.email.size());
    msgpack_pack_str(&g_pk, 3);
    msgpack_pack_str_body(&g_pk, "age", 3);
    msgpack_pack_int32(&g_pk, p.age);

    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_sbuf.data, g_sbuf.size, nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
}

static void BM_Msgpack_Roundtrip_OrderLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& o = GetLargeOrder();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 6);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "order_id", 8);
    msgpack_pack_int64(&g_pk, o.order_id);
    msgpack_pack_str(&g_pk, 11);
    msgpack_pack_str_body(&g_pk, "customer_id", 11);
    msgpack_pack_int64(&g_pk, o.customer_id);
    msgpack_pack_str(&g_pk, 8);
    msgpack_pack_str_body(&g_pk, "item_ids", 8);
    msgpack_pack_array(&g_pk, o.item_ids.size());
    for (auto id : o.item_ids) {
      msgpack_pack_int64(&g_pk, id);
    }
    msgpack_pack_str(&g_pk, 10);
    msgpack_pack_str_body(&g_pk, "quantities", 10);
    msgpack_pack_array(&g_pk, o.quantities.size());
    for (auto q : o.quantities) {
      msgpack_pack_int32(&g_pk, q);
    }
    msgpack_pack_str(&g_pk, 5);
    msgpack_pack_str_body(&g_pk, "total", 5);
    msgpack_pack_double(&g_pk, o.total);
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, o.timestamp);

    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_sbuf.data, g_sbuf.size, nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
}

static void BM_Msgpack_Roundtrip_EventLarge(benchmark::State& state)
{
  init_msgpack_data();
  const auto& e = GetLargeEvent();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    msgpack_pack_map(&g_pk, 5);
    msgpack_pack_str(&g_pk, 2);
    msgpack_pack_str_body(&g_pk, "id", 2);
    msgpack_pack_int64(&g_pk, e.id);
    msgpack_pack_str(&g_pk, 4);
    msgpack_pack_str_body(&g_pk, "type", 4);
    msgpack_pack_str(&g_pk, e.type.size());
    msgpack_pack_str_body(&g_pk, e.type.c_str(), e.type.size());
    msgpack_pack_str(&g_pk, 6);
    msgpack_pack_str_body(&g_pk, "source", 6);
    msgpack_pack_str(&g_pk, e.source.size());
    msgpack_pack_str_body(&g_pk, e.source.c_str(), e.source.size());
    msgpack_pack_str(&g_pk, 9);
    msgpack_pack_str_body(&g_pk, "timestamp", 9);
    msgpack_pack_int64(&g_pk, e.timestamp);
    msgpack_pack_str(&g_pk, 7);
    msgpack_pack_str_body(&g_pk, "payload", 7);
    msgpack_pack_bin(&g_pk, e.payload.size());
    msgpack_pack_bin_body(&g_pk, reinterpret_cast<const char*>(e.payload.data()), e.payload.size());

    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_sbuf.data, g_sbuf.size, nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
}

static void BM_Msgpack_Roundtrip_TreeDeep(benchmark::State& state)
{
  init_msgpack_data();
  const auto& t = GetDeepTree();
  for (auto _ : state) {
    msgpack_sbuffer_clear(&g_sbuf);
    encode_tree_recursive(t);

    msgpack_unpacked result;
    msgpack_unpacked_init(&result);
    msgpack_unpack_next(&result, g_sbuf.data, g_sbuf.size, nullptr);
    benchmark::DoNotOptimize(result.data.via.map.size);
    msgpack_unpacked_destroy(&result);
  }
}

void RegisterMsgpackBenchmarks()
{
  BENCHMARK(BM_Msgpack_Encode_PersonSmall);
  BENCHMARK(BM_Msgpack_Encode_PersonMedium);
  BENCHMARK(BM_Msgpack_Encode_OrderSmall);
  BENCHMARK(BM_Msgpack_Encode_OrderLarge);
  BENCHMARK(BM_Msgpack_Encode_EventSmall);
  BENCHMARK(BM_Msgpack_Encode_EventLarge);
  BENCHMARK(BM_Msgpack_Encode_TreeWide);
  BENCHMARK(BM_Msgpack_Encode_TreeDeep);

  BENCHMARK(BM_Msgpack_Decode_PersonSmall);
  BENCHMARK(BM_Msgpack_Decode_PersonMedium);
  BENCHMARK(BM_Msgpack_Decode_OrderSmall);
  BENCHMARK(BM_Msgpack_Decode_OrderLarge);
  BENCHMARK(BM_Msgpack_Decode_EventSmall);
  BENCHMARK(BM_Msgpack_Decode_EventLarge);
  BENCHMARK(BM_Msgpack_Decode_TreeWide);
  BENCHMARK(BM_Msgpack_Decode_TreeDeep);

  BENCHMARK(BM_Msgpack_Encode_JsonSmall);
  BENCHMARK(BM_Msgpack_Encode_JsonLarge);
  BENCHMARK(BM_Msgpack_Decode_JsonSmall);
  BENCHMARK(BM_Msgpack_Decode_JsonLarge);

  BENCHMARK(BM_Msgpack_Encode_DocumentSmall);
  BENCHMARK(BM_Msgpack_Encode_DocumentLarge);
  BENCHMARK(BM_Msgpack_Decode_DocumentSmall);
  BENCHMARK(BM_Msgpack_Decode_DocumentLarge);

  BENCHMARK(BM_Msgpack_Encode_ChunkedText);
  BENCHMARK(BM_Msgpack_Decode_ChunkedText);

  BENCHMARK(BM_Msgpack_Encode_Embedding768);
  BENCHMARK(BM_Msgpack_Encode_Embedding1536);
  BENCHMARK(BM_Msgpack_Encode_EmbeddingBatch);
  BENCHMARK(BM_Msgpack_Encode_LLMChunkLarge);
  BENCHMARK(BM_Msgpack_Encode_TensorShardLarge);
  BENCHMARK(BM_Msgpack_Encode_InferenceResponse);

  BENCHMARK(BM_Msgpack_Decode_Embedding768);
  BENCHMARK(BM_Msgpack_Decode_Embedding1536);
  BENCHMARK(BM_Msgpack_Decode_EmbeddingBatch);
  BENCHMARK(BM_Msgpack_Decode_LLMChunkLarge);
  BENCHMARK(BM_Msgpack_Decode_TensorShardLarge);
  BENCHMARK(BM_Msgpack_Decode_InferenceResponse);

  BENCHMARK(BM_Msgpack_Roundtrip_PersonSmall);
  BENCHMARK(BM_Msgpack_Roundtrip_OrderLarge);
  BENCHMARK(BM_Msgpack_Roundtrip_EventLarge);
  BENCHMARK(BM_Msgpack_Roundtrip_TreeDeep);
}
