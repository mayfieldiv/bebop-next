#include "encode_helpers.h"
#include "bench_harness.h"

#include <cassert>
#include <cstdlib>
#include <cstring>

#define BEBOP_CHECK(expr, msg) (void)(expr)

static void* libc_alloc(void* ptr, size_t old_size, size_t new_size, void* ctx)
{
  (void)ctx;
  (void)old_size;
  if (new_size == 0) {
    free(ptr);
    return nullptr;
  }
  return realloc(ptr, new_size);
}

static Bebop_WireCtx* g_ctx = nullptr;
static Bebop_Writer* g_writer = nullptr;
static constexpr size_t WRITER_SIZE = 256 * 1024;

static void ensure_ctx()
{
  if (!g_ctx) {
    Bebop_WireCtxOpts opts = Bebop_WireCtx_DefaultOpts();
    opts.arena_options.allocator.alloc = libc_alloc;
    opts.arena_options.allocator.ctx = nullptr;
    opts.arena_options.initial_block_size = 1024 * 1024;
    opts.initial_writer_size = WRITER_SIZE;
    g_ctx = Bebop_WireCtx_New(&opts);
    Bebop_WireCtx_WriterHint(g_ctx, WRITER_SIZE, &g_writer);
  }
}

static std::vector<uint8_t> flusg_writer()
{
  uint8_t* buf;
  size_t len;
  Bebop_Writer_Buf(g_writer, &buf, &len);
  return std::vector<uint8_t>(buf, buf + len);
}

// ---- Person / Order / Event helpers ----

Person make_person(const TestPerson& p)
{
  return Person {
      .name = {.data = p.name.c_str(), .length = static_cast<uint32_t>(p.name.size())},
      .email = {.data = p.email.c_str(), .length = static_cast<uint32_t>(p.email.size())},
      .id = p.id,
      .age = p.age
  };
}

Order make_order(const TestOrder& o)
{
  Bebop_I64_Array item_ids = {
      .data = const_cast<int64_t*>(o.item_ids.data()), .length = o.item_ids.size(), .capacity = 0
  };
  Bebop_I32_Array quantities = {
      .data = const_cast<int32_t*>(o.quantities.data()),
      .length = o.quantities.size(),
      .capacity = 0
  };
  return Order {
      .item_ids = item_ids,
      .quantities = quantities,
      .order_id = o.order_id,
      .customer_id = o.customer_id,
      .total = o.total,
      .timestamp = o.timestamp
  };
}

Event make_event(const TestEvent& e)
{
  Bebop_U8_Array payload = {
      .data = const_cast<uint8_t*>(e.payload.data()), .length = e.payload.size(), .capacity = 0
  };
  return Event {
      .payload = payload,
      .type = {.data = e.type.c_str(), .length = static_cast<uint32_t>(e.type.size())},
      .source = {.data = e.source.c_str(), .length = static_cast<uint32_t>(e.source.size())},
      .id = e.id,
      .timestamp = e.timestamp
  };
}

std::vector<uint8_t> bebop_encode_person_once(const TestPerson& p)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Person person = make_person(p);
  BEBOP_CHECK(Person_Encode(g_writer, &person), "Person_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_order_once(const TestOrder& o)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Order order = make_order(o);
  BEBOP_CHECK(Order_Encode(g_writer, &order), "Order_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_event_once(const TestEvent& e)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Event event = make_event(e);
  BEBOP_CHECK(Event_Encode(g_writer, &event), "Event_Encode");
  return flusg_writer();
}

// ---- Tree helpers ----

void convert_tree_recursive(
    const TestTreeNode& src,
    TreeNode& dst,
    std::vector<TreeNode>& nodes,
    std::vector<TreeNode_Array>& children
)
{
  dst = {};
  BEBOP_WIRE_SET_SOME(dst.value, src.value);

  if (!src.children.empty()) {
    size_t start_idx = nodes.size();
    for (size_t i = 0; i < src.children.size(); i++) {
      TreeNode child_node {};
      nodes.push_back(child_node);
    }
    for (size_t i = 0; i < src.children.size(); i++) {
      convert_tree_recursive(src.children[i], nodes[start_idx + i], nodes, children);
    }
    TreeNode_Array arr = {.data = &nodes[start_idx], .length = src.children.size(), .capacity = 0};
    children.push_back(arr);
    BEBOP_WIRE_SET_SOME(dst.children, children.back());
  }
}

static std::vector<uint8_t> encode_tree_once(const TestTreeNode& src)
{
  ensure_ctx();

  std::vector<TreeNode> nodes;
  std::vector<TreeNode_Array> child_arrays;
  nodes.reserve(2000);
  child_arrays.reserve(1000);
  TreeNode* nodes_base = nodes.data();
  TreeNode_Array* children_base = child_arrays.data();

  TreeNode root;
  convert_tree_recursive(src, root, nodes, child_arrays);

  assert(nodes.data() == nodes_base && "nodes reserve exceeded — increase reserve");
  assert(child_arrays.data() == children_base && "child_arrays reserve exceeded — increase reserve");

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(TreeNode_Encode(g_writer, &root), "TreeNode_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_tree_wide_once()
{
  return encode_tree_once(GetWideTree());
}

std::vector<uint8_t> bebop_encode_tree_deep_once()
{
  return encode_tree_once(GetDeepTree());
}

// ---- JSON / Document helpers ----

JsonValue convert_json_value(
    Bebop_WireCtx* ctx,
    const TestJsonValue& src,
    std::vector<JsonValue>& storage,
    std::vector<JsonValue_Array>& arrays,
    std::vector<Bebop_Map>& maps
)
{
  JsonValue v = {};
  switch (src.type) {
    case TestJsonValue::Type::Null:
      v.discriminator = JSON_VALUE_NULL;
      v.null = {};
      break;
    case TestJsonValue::Type::Bool:
      v.discriminator = JSON_VALUE_BOOL;
      BEBOP_WIRE_SET_SOME(v.bool_.value, src.bool_val);
      break;
    case TestJsonValue::Type::Number:
      v.discriminator = JSON_VALUE_NUMBER;
      BEBOP_WIRE_SET_SOME(v.number.value, src.number_val);
      break;
    case TestJsonValue::Type::String:
      v.discriminator = JSON_VALUE_STRING;
      BEBOP_WIRE_SET_SOME(
          v.string.value,
          ((Bebop_Str) {.data = src.string_val.c_str(),
                        .length = static_cast<uint32_t>(src.string_val.size())})
      );
      break;
    case TestJsonValue::Type::List: {
      v.discriminator = JSON_VALUE_LIST;
      size_t start = storage.size();
      storage.resize(start + src.list_val.size());
      for (size_t i = 0; i < src.list_val.size(); i++) {
        storage[start + i] = convert_json_value(ctx, src.list_val[i], storage, arrays, maps);
      }
      JsonValue_Array arr = {
          .data = storage.data() + start, .length = src.list_val.size(), .capacity = 0
      };
      arrays.push_back(arr);
      BEBOP_WIRE_SET_SOME(v.list.values, arrays.back());
      break;
    }
    case TestJsonValue::Type::Object: {
      v.discriminator = JSON_VALUE_OBJECT;
      Bebop_Map m = {};
      Bebop_Map_Init(&m, ctx, Bebop_MapHash_Str, Bebop_MapEq_Str);
      for (const auto& [key, val] : src.object_val) {
        Bebop_Str* key_ptr = static_cast<Bebop_Str*>(Bebop_WireCtx_Alloc(ctx, sizeof(Bebop_Str)));
        key_ptr->data = key.c_str();
        key_ptr->length = static_cast<uint32_t>(key.size());
        JsonValue* val_ptr = static_cast<JsonValue*>(Bebop_WireCtx_Alloc(ctx, sizeof(JsonValue)));
        *val_ptr = convert_json_value(ctx, val, storage, arrays, maps);
        Bebop_Map_Put(&m, key_ptr, val_ptr);
      }
      maps.push_back(m);
      BEBOP_WIRE_SET_SOME(v.object.fields, maps.back());
      break;
    }
  }
  return v;
}

Document make_document(
    Bebop_WireCtx* ctx,
    const TestDocument& d,
    std::vector<JsonValue>& storage,
    std::vector<JsonValue_Array>& arrays,
    std::vector<Bebop_Map>& maps
)
{
  Document doc = {};
  if (!d.title.empty()) {
    BEBOP_WIRE_SET_SOME(
        doc.title,
        ((Bebop_Str) {.data = d.title.c_str(), .length = static_cast<uint32_t>(d.title.size())})
    );
  }
  if (!d.body.empty()) {
    BEBOP_WIRE_SET_SOME(
        doc.body,
        ((Bebop_Str) {.data = d.body.c_str(), .length = static_cast<uint32_t>(d.body.size())})
    );
  }
  if (!d.metadata.empty()) {
    Bebop_Map m = {};
    Bebop_Map_Init(&m, ctx, Bebop_MapHash_Str, Bebop_MapEq_Str);
    for (const auto& [key, val] : d.metadata) {
      Bebop_Str* key_ptr = static_cast<Bebop_Str*>(Bebop_WireCtx_Alloc(ctx, sizeof(Bebop_Str)));
      key_ptr->data = key.c_str();
      key_ptr->length = static_cast<uint32_t>(key.size());
      JsonValue* val_ptr = static_cast<JsonValue*>(Bebop_WireCtx_Alloc(ctx, sizeof(JsonValue)));
      *val_ptr = convert_json_value(ctx, val, storage, arrays, maps);
      Bebop_Map_Put(&m, key_ptr, val_ptr);
    }
    maps.push_back(m);
    BEBOP_WIRE_SET_SOME(doc.metadata, maps.back());
  }
  return doc;
}

static std::vector<uint8_t> encode_json_once(const TestJsonValue& src)
{
  ensure_ctx();

  std::vector<JsonValue> storage;
  std::vector<JsonValue_Array> arrays;
  std::vector<Bebop_Map> maps;
  storage.reserve(1000);
  arrays.reserve(100);
  maps.reserve(50);
  JsonValue* storage_base = storage.data();
  JsonValue_Array* arrays_base = arrays.data();
  Bebop_Map* maps_base = maps.data();

  JsonValue val = convert_json_value(g_ctx, src, storage, arrays, maps);

  assert(storage.data() == storage_base && "storage reserve exceeded — increase reserve");
  assert(arrays.data() == arrays_base && "arrays reserve exceeded — increase reserve");
  assert(maps.data() == maps_base && "maps reserve exceeded — increase reserve");
  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(JsonValue_Encode(g_writer, &val), "JsonValue_Encode");
  return flusg_writer();
}

static std::vector<uint8_t> encode_doc_once(const TestDocument& d)
{
  ensure_ctx();

  std::vector<JsonValue> storage;
  std::vector<JsonValue_Array> arrays;
  std::vector<Bebop_Map> maps;
  storage.reserve(1000);
  arrays.reserve(100);
  maps.reserve(50);
  JsonValue* storage_base = storage.data();
  JsonValue_Array* arrays_base = arrays.data();
  Bebop_Map* maps_base = maps.data();

  Document doc = make_document(g_ctx, d, storage, arrays, maps);

  assert(storage.data() == storage_base && "storage reserve exceeded — increase reserve");
  assert(arrays.data() == arrays_base && "arrays reserve exceeded — increase reserve");
  assert(maps.data() == maps_base && "maps reserve exceeded — increase reserve");
  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(Document_Encode(g_writer, &doc), "Document_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_json_small_once()
{
  return encode_json_once(GetSmallJson());
}

std::vector<uint8_t> bebop_encode_json_large_once()
{
  return encode_json_once(GetLargeJson());
}

std::vector<uint8_t> bebop_encode_doc_small_once()
{
  return encode_doc_once(GetSmallDocument());
}

std::vector<uint8_t> bebop_encode_doc_large_once()
{
  return encode_doc_once(GetLargeDocument());
}

// ---- ChunkedText helpers ----

std::vector<uint8_t> bebop_encode_chunked_text_once()
{
  ensure_ctx();

  const auto& src = GetAliceChunks();

  std::vector<TextSpan> spans;
  spans.reserve(src.spans.size());
  for (const auto& span : src.spans) {
    spans.push_back(
        TextSpan {.kind = static_cast<ChunkKind>(span.kind), .start = span.start, .len = span.len}
    );
  }

  ChunkedText ct = {
      .spans = {.data = spans.data(), .length = spans.size(), .capacity = 0},
      .source = {.data = src.source.c_str(), .length = static_cast<uint32_t>(src.source.size())}
  };

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(ChunkedText_Encode(g_writer, &ct), "ChunkedText_Encode");
  return flusg_writer();
}

// ---- AI/Embedding helpers ----

EmbeddingBF16 make_embedding_bf16(const TestEmbeddingBF16& e)
{
  return EmbeddingBF16 {
      .vector =
          {.data = reinterpret_cast<Bebop_BFloat16*>(const_cast<uint16_t*>(e.vector.data())),
           .length = e.vector.size(),
           .capacity = 0},
      .id = *reinterpret_cast<const Bebop_UUID*>(e.id.bytes)
  };
}

EmbeddingF32 make_embedding_f32(const TestEmbeddingF32& e)
{
  return EmbeddingF32 {
      .vector =
          {.data = const_cast<float*>(e.vector.data()), .length = e.vector.size(), .capacity = 0},
      .id = *reinterpret_cast<const Bebop_UUID*>(e.id.bytes)
  };
}

static std::vector<uint8_t> encode_embedding_bf16_once(const TestEmbeddingBF16& e)
{
  ensure_ctx();
  EmbeddingBF16 emb = make_embedding_bf16(e);
  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(EmbeddingBF16_Encode(g_writer, &emb), "EmbeddingBF16_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_embedding384_once()
{
  return encode_embedding_bf16_once(GetEmbedding384());
}

std::vector<uint8_t> bebop_encode_embedding768_once()
{
  return encode_embedding_bf16_once(GetEmbedding768());
}

std::vector<uint8_t> bebop_encode_embedding1536_once()
{
  return encode_embedding_bf16_once(GetEmbedding1536());
}

std::vector<uint8_t> bebop_encode_embedding_f32_768_once()
{
  ensure_ctx();
  EmbeddingF32 emb = make_embedding_f32(GetEmbeddingF32_768());
  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(EmbeddingF32_Encode(g_writer, &emb), "EmbeddingF32_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_embedding_batch_once()
{
  ensure_ctx();

  const auto& batch = GetEmbeddingBatch();
  std::vector<EmbeddingBF16> batch_embeddings;
  for (const auto& e : batch.embeddings) {
    batch_embeddings.push_back(make_embedding_bf16(e));
  }
  EmbeddingBatch eb;
  eb.model = {batch.model.c_str(), static_cast<uint32_t>(batch.model.size())};
  eb.embeddings = {batch_embeddings.data(), batch_embeddings.size(), 0};
  eb.usage_tokens = batch.usage_tokens;

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(EmbeddingBatch_Encode(g_writer, &eb), "EmbeddingBatch_Encode");
  return flusg_writer();
}

static std::vector<uint8_t> encode_llm_chunk_once(const TestLLMStreamChunk& llm)
{
  ensure_ctx();

  std::vector<Bebop_Str> tokens;
  for (const auto& t : llm.tokens) {
    tokens.push_back({t.c_str(), static_cast<uint32_t>(t.size())});
  }
  std::vector<std::vector<TokenLogprob>> logprobs_storage;
  std::vector<TokenAlternatives> alts;
  for (const auto& alt : llm.logprobs) {
    std::vector<TokenLogprob> toks;
    for (const auto& lp : alt.top_tokens) {
      toks.push_back(
          {{lp.token.c_str(), static_cast<uint32_t>(lp.token.size())}, lp.token_id, lp.logprob}
      );
    }
    logprobs_storage.push_back(toks);
    alts.push_back(
        {{logprobs_storage.back().data(), logprobs_storage.back().size(), 0}}
    );
  }
  LLMStreamChunk chunk;
  chunk.chunk_id = llm.chunk_id;
  chunk.tokens = {tokens.data(), tokens.size(), 0};
  chunk.logprobs = {alts.data(), alts.size(), 0};
  chunk.finish_reason = {
      llm.finish_reason.c_str(), static_cast<uint32_t>(llm.finish_reason.size())
  };

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(LLMStreamChunk_Encode(g_writer, &chunk), "LLMStreamChunk_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_llm_chunk_small_once()
{
  return encode_llm_chunk_once(GetLLMChunkSmall());
}

std::vector<uint8_t> bebop_encode_llm_chunk_large_once()
{
  return encode_llm_chunk_once(GetLLMChunkLarge());
}

static std::vector<uint8_t> encode_tensor_shard_once(const TestTensorShard& ts)
{
  ensure_ctx();

  TensorShard shard;
  shard.name = {ts.name.c_str(), static_cast<uint32_t>(ts.name.size())};
  shard.shape = {const_cast<uint32_t*>(ts.shape.data()), ts.shape.size(), 0};
  shard.dtype = {ts.dtype.c_str(), static_cast<uint32_t>(ts.dtype.size())};
  shard.data = {
      reinterpret_cast<Bebop_BFloat16*>(const_cast<uint16_t*>(ts.data.data())), ts.data.size(), 0
  };
  shard.offset = ts.offset;
  shard.total_elements = ts.total_elements;

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(TensorShard_Encode(g_writer, &shard), "TensorShard_Encode");
  return flusg_writer();
}

std::vector<uint8_t> bebop_encode_tensor_shard_small_once()
{
  return encode_tensor_shard_once(GetTensorShardSmall());
}

std::vector<uint8_t> bebop_encode_tensor_shard_large_once()
{
  return encode_tensor_shard_once(GetTensorShardLarge());
}

std::vector<uint8_t> bebop_encode_inference_response_once()
{
  ensure_ctx();

  const auto& inf = GetInferenceResponse();
  std::vector<EmbeddingBF16> embeddings;
  for (const auto& e : inf.embeddings) {
    embeddings.push_back(make_embedding_bf16(e));
  }
  InferenceResponse resp;
  resp.request_id = *reinterpret_cast<const Bebop_UUID*>(inf.request_id.bytes);
  resp.embeddings = {embeddings.data(), embeddings.size(), 0};
  resp.timing.queue_time = {inf.timing.queue_time.seconds, inf.timing.queue_time.nanos};
  resp.timing.inference_time = {
      inf.timing.inference_time.seconds, inf.timing.inference_time.nanos
  };
  resp.timing.tokens_per_second = inf.timing.tokens_per_second;

  Bebop_Writer_Reset(g_writer);
  BEBOP_CHECK(InferenceResponse_Encode(g_writer, &resp), "InferenceResponse_Encode");
  return flusg_writer();
}
