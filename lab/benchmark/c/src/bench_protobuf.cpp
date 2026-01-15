#include <benchmark/benchmark.h>

#include "bench_harness.h"

extern "C" {
#include "benchmark.pb-c.h"
}

#include <cstdlib>
#include <cstring>
#include <vector>

static Benchmark__Person* make_person(const TestPerson& p)
{
  auto* msg = static_cast<Benchmark__Person*>(malloc(sizeof(Benchmark__Person)));
  benchmark__person__init(msg);
  msg->id = p.id;
  msg->name = const_cast<char*>(p.name.c_str());
  msg->email = const_cast<char*>(p.email.c_str());
  msg->age = p.age;
  return msg;
}

static Benchmark__Order* make_order(const TestOrder& o)
{
  auto* msg = static_cast<Benchmark__Order*>(malloc(sizeof(Benchmark__Order)));
  benchmark__order__init(msg);
  msg->order_id = o.order_id;
  msg->customer_id = o.customer_id;
  msg->n_item_ids = o.item_ids.size();
  msg->item_ids = const_cast<int64_t*>(o.item_ids.data());
  msg->n_quantities = o.quantities.size();
  msg->quantities = const_cast<int32_t*>(o.quantities.data());
  msg->total = o.total;
  msg->timestamp = o.timestamp;
  return msg;
}

static Benchmark__Event* make_event(const TestEvent& e)
{
  auto* msg = static_cast<Benchmark__Event*>(malloc(sizeof(Benchmark__Event)));
  benchmark__event__init(msg);
  msg->id = e.id;
  msg->type = const_cast<char*>(e.type.c_str());
  msg->source = const_cast<char*>(e.source.c_str());
  msg->timestamp = e.timestamp;
  msg->payload.len = e.payload.size();
  msg->payload.data = const_cast<uint8_t*>(e.payload.data());
  return msg;
}

static std::vector<Benchmark__TreeNode*> g_tree_nodes;

static Benchmark__TreeNode* make_tree(const TestTreeNode& t)
{
  auto* msg = static_cast<Benchmark__TreeNode*>(malloc(sizeof(Benchmark__TreeNode)));
  benchmark__tree_node__init(msg);
  msg->value = t.value;
  if (!t.children.empty()) {
    msg->n_children = t.children.size();
    msg->children = static_cast<Benchmark__TreeNode**>(
        malloc(sizeof(Benchmark__TreeNode*) * t.children.size())
    );
    for (size_t i = 0; i < t.children.size(); i++) {
      msg->children[i] = make_tree(t.children[i]);
      g_tree_nodes.push_back(msg->children[i]);
    }
  }
  return msg;
}

static std::vector<Benchmark__JsonValue*> g_json_values;
static std::vector<Benchmark__JsonList*> g_json_lists;
static std::vector<Benchmark__JsonObject*> g_json_objects;
static std::vector<Benchmark__JsonObject__FieldsEntry*> g_json_entries;

static Benchmark__JsonValue* make_json_value(const TestJsonValue& v);

static Benchmark__JsonValue* make_json_value(const TestJsonValue& v)
{
  auto* msg = static_cast<Benchmark__JsonValue*>(malloc(sizeof(Benchmark__JsonValue)));
  benchmark__json_value__init(msg);

  switch (v.type) {
    case TestJsonValue::Type::Null: {
      auto* null_val = static_cast<Benchmark__JsonNull*>(malloc(sizeof(Benchmark__JsonNull)));
      benchmark__json_null__init(null_val);
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_NULL_VAL;
      msg->null_val = null_val;
      break;
    }
    case TestJsonValue::Type::Bool: {
      auto* bool_val = static_cast<Benchmark__JsonBool*>(malloc(sizeof(Benchmark__JsonBool)));
      benchmark__json_bool__init(bool_val);
      bool_val->value = v.bool_val;
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_BOOL_VAL;
      msg->bool_val = bool_val;
      break;
    }
    case TestJsonValue::Type::Number: {
      auto* num_val = static_cast<Benchmark__JsonNumber*>(malloc(sizeof(Benchmark__JsonNumber)));
      benchmark__json_number__init(num_val);
      num_val->value = v.number_val;
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_NUMBER_VAL;
      msg->number_val = num_val;
      break;
    }
    case TestJsonValue::Type::String: {
      auto* str_val = static_cast<Benchmark__JsonString*>(malloc(sizeof(Benchmark__JsonString)));
      benchmark__json_string__init(str_val);
      str_val->value = const_cast<char*>(v.string_val.c_str());
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_STRING_VAL;
      msg->string_val = str_val;
      break;
    }
    case TestJsonValue::Type::List: {
      auto* list_val = static_cast<Benchmark__JsonList*>(malloc(sizeof(Benchmark__JsonList)));
      benchmark__json_list__init(list_val);
      list_val->n_values = v.list_val.size();
      if (!v.list_val.empty()) {
        list_val->values = static_cast<Benchmark__JsonValue**>(
            malloc(sizeof(Benchmark__JsonValue*) * v.list_val.size())
        );
        for (size_t i = 0; i < v.list_val.size(); i++) {
          list_val->values[i] = make_json_value(v.list_val[i]);
          g_json_values.push_back(list_val->values[i]);
        }
      }
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_LIST_VAL;
      msg->list_val = list_val;
      g_json_lists.push_back(list_val);
      break;
    }
    case TestJsonValue::Type::Object: {
      auto* obj_val = static_cast<Benchmark__JsonObject*>(malloc(sizeof(Benchmark__JsonObject)));
      benchmark__json_object__init(obj_val);
      obj_val->n_fields = v.object_val.size();
      if (!v.object_val.empty()) {
        obj_val->fields = static_cast<Benchmark__JsonObject__FieldsEntry**>(
            malloc(sizeof(Benchmark__JsonObject__FieldsEntry*) * v.object_val.size())
        );
        for (size_t i = 0; i < v.object_val.size(); i++) {
          auto* entry = static_cast<Benchmark__JsonObject__FieldsEntry*>(
              malloc(sizeof(Benchmark__JsonObject__FieldsEntry))
          );
          benchmark__json_object__fields_entry__init(entry);
          entry->key = const_cast<char*>(v.object_val[i].first.c_str());
          entry->value = make_json_value(v.object_val[i].second);
          g_json_values.push_back(entry->value);
          obj_val->fields[i] = entry;
          g_json_entries.push_back(entry);
        }
      }
      msg->value_case = BENCHMARK__JSON_VALUE__VALUE_OBJECT_VAL;
      msg->object_val = obj_val;
      g_json_objects.push_back(obj_val);
      break;
    }
  }
  return msg;
}

static Benchmark__Document* make_document(const TestDocument& d)
{
  auto* msg = static_cast<Benchmark__Document*>(malloc(sizeof(Benchmark__Document)));
  benchmark__document__init(msg);
  msg->title = const_cast<char*>(d.title.c_str());
  msg->body = const_cast<char*>(d.body.c_str());
  msg->n_metadata = d.metadata.size();
  if (!d.metadata.empty()) {
    msg->metadata = static_cast<Benchmark__Document__MetadataEntry**>(
        malloc(sizeof(Benchmark__Document__MetadataEntry*) * d.metadata.size())
    );
    for (size_t i = 0; i < d.metadata.size(); i++) {
      auto* entry = static_cast<Benchmark__Document__MetadataEntry*>(
          malloc(sizeof(Benchmark__Document__MetadataEntry))
      );
      benchmark__document__metadata_entry__init(entry);
      entry->key = const_cast<char*>(d.metadata[i].first.c_str());
      entry->value = make_json_value(d.metadata[i].second);
      g_json_values.push_back(entry->value);
      msg->metadata[i] = entry;
    }
  }
  return msg;
}

static std::vector<Benchmark__TextSpan*> g_text_spans_proto;

static Benchmark__ChunkedText* make_chunked_text(const TestChunkedText& c)
{
  auto* msg = static_cast<Benchmark__ChunkedText*>(malloc(sizeof(Benchmark__ChunkedText)));
  benchmark__chunked_text__init(msg);
  msg->source = const_cast<char*>(c.source.c_str());
  msg->n_spans = c.spans.size();
  if (!c.spans.empty()) {
    msg->spans =
        static_cast<Benchmark__TextSpan**>(malloc(sizeof(Benchmark__TextSpan*) * c.spans.size()));
    for (size_t i = 0; i < c.spans.size(); i++) {
      auto* span = static_cast<Benchmark__TextSpan*>(malloc(sizeof(Benchmark__TextSpan)));
      benchmark__text_span__init(span);
      span->start = c.spans[i].start;
      span->len = c.spans[i].len;
      span->kind = static_cast<Benchmark__ChunkKind>(c.spans[i].kind);
      msg->spans[i] = span;
      g_text_spans_proto.push_back(span);
    }
  }
  return msg;
}

static std::string uuid_to_string(const TestUUID& u)
{
  char buf[37];
  snprintf(
      buf,
      sizeof(buf),
      "%02x%02x%02x%02x-%02x%02x-%02x%02x-%02x%02x-%02x%02x%02x%02x%02x%02x",
      u.bytes[0],
      u.bytes[1],
      u.bytes[2],
      u.bytes[3],
      u.bytes[4],
      u.bytes[5],
      u.bytes[6],
      u.bytes[7],
      u.bytes[8],
      u.bytes[9],
      u.bytes[10],
      u.bytes[11],
      u.bytes[12],
      u.bytes[13],
      u.bytes[14],
      u.bytes[15]
  );
  return buf;
}

static std::vector<std::string> g_uuid_strings;
static std::vector<Benchmark__EmbeddingBF16*> g_proto_embeddings;
static std::vector<Benchmark__TokenLogprob*> g_proto_logprobs;
static std::vector<Benchmark__TokenAlternatives*> g_proto_alts;

static Benchmark__EmbeddingBF16* make_embedding_bf16(const TestEmbeddingBF16& e)
{
  auto* msg = static_cast<Benchmark__EmbeddingBF16*>(malloc(sizeof(Benchmark__EmbeddingBF16)));
  benchmark__embedding_bf16__init(msg);
  g_uuid_strings.push_back(uuid_to_string(e.id));
  msg->id = const_cast<char*>(g_uuid_strings.back().c_str());
  msg->vector.len = e.vector.size() * 2;
  msg->vector.data = reinterpret_cast<uint8_t*>(const_cast<uint16_t*>(e.vector.data()));
  return msg;
}

static Benchmark__EmbeddingF32* make_embedding_f32(const TestEmbeddingF32& e)
{
  auto* msg = static_cast<Benchmark__EmbeddingF32*>(malloc(sizeof(Benchmark__EmbeddingF32)));
  benchmark__embedding_f32__init(msg);
  g_uuid_strings.push_back(uuid_to_string(e.id));
  msg->id = const_cast<char*>(g_uuid_strings.back().c_str());
  msg->n_vector = e.vector.size();
  msg->vector = const_cast<float*>(e.vector.data());
  return msg;
}

static Benchmark__EmbeddingBatch* make_embedding_batch(const TestEmbeddingBatch& b)
{
  auto* msg = static_cast<Benchmark__EmbeddingBatch*>(malloc(sizeof(Benchmark__EmbeddingBatch)));
  benchmark__embedding_batch__init(msg);
  msg->model = const_cast<char*>(b.model.c_str());
  msg->n_embeddings = b.embeddings.size();
  msg->embeddings = static_cast<Benchmark__EmbeddingBF16**>(
      malloc(sizeof(Benchmark__EmbeddingBF16*) * b.embeddings.size())
  );
  for (size_t i = 0; i < b.embeddings.size(); i++) {
    msg->embeddings[i] = make_embedding_bf16(b.embeddings[i]);
    g_proto_embeddings.push_back(msg->embeddings[i]);
  }
  msg->usage_tokens = b.usage_tokens;
  return msg;
}

static Benchmark__LLMStreamChunk* make_llm_chunk(const TestLLMStreamChunk& c)
{
  auto* msg = static_cast<Benchmark__LLMStreamChunk*>(malloc(sizeof(Benchmark__LLMStreamChunk)));
  benchmark__llmstream_chunk__init(msg);
  msg->chunk_id = c.chunk_id;
  msg->n_tokens = c.tokens.size();
  msg->tokens = static_cast<char**>(malloc(sizeof(char*) * c.tokens.size()));
  for (size_t i = 0; i < c.tokens.size(); i++) {
    msg->tokens[i] = const_cast<char*>(c.tokens[i].c_str());
  }
  msg->n_logprobs = c.logprobs.size();
  msg->logprobs = static_cast<Benchmark__TokenAlternatives**>(
      malloc(sizeof(Benchmark__TokenAlternatives*) * c.logprobs.size())
  );
  for (size_t i = 0; i < c.logprobs.size(); i++) {
    auto* alt =
        static_cast<Benchmark__TokenAlternatives*>(malloc(sizeof(Benchmark__TokenAlternatives)));
    benchmark__token_alternatives__init(alt);
    alt->n_top_tokens = c.logprobs[i].top_tokens.size();
    alt->top_tokens = static_cast<Benchmark__TokenLogprob**>(
        malloc(sizeof(Benchmark__TokenLogprob*) * c.logprobs[i].top_tokens.size())
    );
    for (size_t j = 0; j < c.logprobs[i].top_tokens.size(); j++) {
      auto* lp = static_cast<Benchmark__TokenLogprob*>(malloc(sizeof(Benchmark__TokenLogprob)));
      benchmark__token_logprob__init(lp);
      lp->token = const_cast<char*>(c.logprobs[i].top_tokens[j].token.c_str());
      lp->token_id = c.logprobs[i].top_tokens[j].token_id;
      lp->logprob = c.logprobs[i].top_tokens[j].logprob;
      alt->top_tokens[j] = lp;
      g_proto_logprobs.push_back(lp);
    }
    msg->logprobs[i] = alt;
    g_proto_alts.push_back(alt);
  }
  msg->finish_reason = const_cast<char*>(c.finish_reason.c_str());
  return msg;
}

static Benchmark__TensorShard* make_tensor_shard(const TestTensorShard& t)
{
  auto* msg = static_cast<Benchmark__TensorShard*>(malloc(sizeof(Benchmark__TensorShard)));
  benchmark__tensor_shard__init(msg);
  msg->name = const_cast<char*>(t.name.c_str());
  msg->n_shape = t.shape.size();
  msg->shape = const_cast<uint32_t*>(t.shape.data());
  msg->dtype = const_cast<char*>(t.dtype.c_str());
  msg->data.len = t.data.size() * 2;
  msg->data.data = reinterpret_cast<uint8_t*>(const_cast<uint16_t*>(t.data.data()));
  msg->offset = t.offset;
  msg->total_elements = t.total_elements;
  return msg;
}

static Google__Protobuf__Duration* g_queue_duration = nullptr;
static Google__Protobuf__Duration* g_inference_duration = nullptr;

static Benchmark__InferenceResponse* make_inference_response(const TestInferenceResponse& r)
{
  auto* msg =
      static_cast<Benchmark__InferenceResponse*>(malloc(sizeof(Benchmark__InferenceResponse)));
  benchmark__inference_response__init(msg);
  g_uuid_strings.push_back(uuid_to_string(r.request_id));
  msg->request_id = const_cast<char*>(g_uuid_strings.back().c_str());
  msg->n_embeddings = r.embeddings.size();
  msg->embeddings = static_cast<Benchmark__EmbeddingBF16**>(
      malloc(sizeof(Benchmark__EmbeddingBF16*) * r.embeddings.size())
  );
  for (size_t i = 0; i < r.embeddings.size(); i++) {
    msg->embeddings[i] = make_embedding_bf16(r.embeddings[i]);
    g_proto_embeddings.push_back(msg->embeddings[i]);
  }
  auto* timing =
      static_cast<Benchmark__InferenceTiming*>(malloc(sizeof(Benchmark__InferenceTiming)));
  benchmark__inference_timing__init(timing);
  g_queue_duration =
      static_cast<Google__Protobuf__Duration*>(malloc(sizeof(Google__Protobuf__Duration)));
  google__protobuf__duration__init(g_queue_duration);
  g_queue_duration->seconds = r.timing.queue_time.seconds;
  g_queue_duration->nanos = r.timing.queue_time.nanos;
  timing->queue_time = g_queue_duration;
  g_inference_duration =
      static_cast<Google__Protobuf__Duration*>(malloc(sizeof(Google__Protobuf__Duration)));
  google__protobuf__duration__init(g_inference_duration);
  g_inference_duration->seconds = r.timing.inference_time.seconds;
  g_inference_duration->nanos = r.timing.inference_time.nanos;
  timing->inference_time = g_inference_duration;
  timing->tokens_per_second = r.timing.tokens_per_second;
  msg->timing = timing;
  return msg;
}

static Benchmark__Person* g_small_person = nullptr;
static Benchmark__Person* g_medium_person = nullptr;
static Benchmark__Order* g_small_order = nullptr;
static Benchmark__Order* g_large_order = nullptr;
static Benchmark__Event* g_small_event = nullptr;
static Benchmark__Event* g_large_event = nullptr;
static Benchmark__TreeNode* g_wide_tree = nullptr;
static Benchmark__TreeNode* g_deep_tree = nullptr;
static Benchmark__JsonValue* g_small_json = nullptr;
static Benchmark__JsonValue* g_large_json = nullptr;
static Benchmark__Document* g_small_document = nullptr;
static Benchmark__Document* g_large_document = nullptr;
static Benchmark__ChunkedText* g_alice_chunks = nullptr;
static Benchmark__EmbeddingBF16* g_emb_768 = nullptr;
static Benchmark__EmbeddingBF16* g_emb_1536 = nullptr;
static Benchmark__EmbeddingF32* g_emb_f32_768 = nullptr;
static Benchmark__EmbeddingBatch* g_emb_batch = nullptr;
static Benchmark__LLMStreamChunk* g_llm_small = nullptr;
static Benchmark__LLMStreamChunk* g_llm_large = nullptr;
static Benchmark__TensorShard* g_tensor_small = nullptr;
static Benchmark__TensorShard* g_tensor_large = nullptr;
static Benchmark__InferenceResponse* g_inference = nullptr;

static std::vector<uint8_t> g_encoded_person_small;
static std::vector<uint8_t> g_encoded_person_medium;
static std::vector<uint8_t> g_encoded_order_small;
static std::vector<uint8_t> g_encoded_order_large;
static std::vector<uint8_t> g_encoded_event_small;
static std::vector<uint8_t> g_encoded_event_large;
static std::vector<uint8_t> g_encoded_tree_wide;
static std::vector<uint8_t> g_encoded_tree_deep;
static std::vector<uint8_t> g_encoded_json_small;
static std::vector<uint8_t> g_encoded_json_large;
static std::vector<uint8_t> g_encoded_doc_small;
static std::vector<uint8_t> g_encoded_doc_large;
static std::vector<uint8_t> g_encoded_alice;
static std::vector<uint8_t> g_encoded_emb_768;
static std::vector<uint8_t> g_encoded_emb_1536;
static std::vector<uint8_t> g_encoded_emb_f32_768;
static std::vector<uint8_t> g_encoded_emb_batch;
static std::vector<uint8_t> g_encoded_llm_small;
static std::vector<uint8_t> g_encoded_llm_large;
static std::vector<uint8_t> g_encoded_tensor_small;
static std::vector<uint8_t> g_encoded_tensor_large;
static std::vector<uint8_t> g_encoded_inference;

static std::vector<uint8_t> g_encode_buffer;
static bool g_initialized = false;

static void init_protobuf_data()
{
  if (g_initialized) {
    return;
  }

  g_small_person = make_person(GetSmallPerson());
  g_medium_person = make_person(GetMediumPerson());
  g_small_order = make_order(GetSmallOrder());
  g_large_order = make_order(GetLargeOrder());
  g_small_event = make_event(GetSmallEvent());
  g_large_event = make_event(GetLargeEvent());
  g_wide_tree = make_tree(GetWideTree());
  g_deep_tree = make_tree(GetDeepTree());
  g_small_json = make_json_value(GetSmallJson());
  g_large_json = make_json_value(GetLargeJson());
  g_small_document = make_document(GetSmallDocument());
  g_large_document = make_document(GetLargeDocument());
  g_alice_chunks = make_chunked_text(GetAliceChunks());

  g_encode_buffer.resize(1024 * 1024);

  size_t len;

  len = benchmark__person__pack(g_small_person, g_encode_buffer.data());
  g_encoded_person_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__person__pack(g_medium_person, g_encode_buffer.data());
  g_encoded_person_medium.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__order__pack(g_small_order, g_encode_buffer.data());
  g_encoded_order_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__order__pack(g_large_order, g_encode_buffer.data());
  g_encoded_order_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__event__pack(g_small_event, g_encode_buffer.data());
  g_encoded_event_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__event__pack(g_large_event, g_encode_buffer.data());
  g_encoded_event_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__tree_node__pack(g_wide_tree, g_encode_buffer.data());
  g_encoded_tree_wide.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__tree_node__pack(g_deep_tree, g_encode_buffer.data());
  g_encoded_tree_deep.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__json_value__pack(g_small_json, g_encode_buffer.data());
  g_encoded_json_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__json_value__pack(g_large_json, g_encode_buffer.data());
  g_encoded_json_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__document__pack(g_small_document, g_encode_buffer.data());
  g_encoded_doc_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__document__pack(g_large_document, g_encode_buffer.data());
  g_encoded_doc_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__chunked_text__pack(g_alice_chunks, g_encode_buffer.data());
  g_encoded_alice.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  g_emb_768 = make_embedding_bf16(GetEmbedding768());
  g_emb_1536 = make_embedding_bf16(GetEmbedding1536());
  g_emb_f32_768 = make_embedding_f32(GetEmbeddingF32_768());
  g_emb_batch = make_embedding_batch(GetEmbeddingBatch());
  g_llm_small = make_llm_chunk(GetLLMChunkSmall());
  g_llm_large = make_llm_chunk(GetLLMChunkLarge());
  g_tensor_small = make_tensor_shard(GetTensorShardSmall());
  g_tensor_large = make_tensor_shard(GetTensorShardLarge());
  g_inference = make_inference_response(GetInferenceResponse());

  len = benchmark__embedding_bf16__pack(g_emb_768, g_encode_buffer.data());
  g_encoded_emb_768.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__embedding_bf16__pack(g_emb_1536, g_encode_buffer.data());
  g_encoded_emb_1536.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__embedding_f32__pack(g_emb_f32_768, g_encode_buffer.data());
  g_encoded_emb_f32_768.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__embedding_batch__pack(g_emb_batch, g_encode_buffer.data());
  g_encoded_emb_batch.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__llmstream_chunk__pack(g_llm_small, g_encode_buffer.data());
  g_encoded_llm_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__llmstream_chunk__pack(g_llm_large, g_encode_buffer.data());
  g_encoded_llm_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__tensor_shard__pack(g_tensor_small, g_encode_buffer.data());
  g_encoded_tensor_small.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__tensor_shard__pack(g_tensor_large, g_encode_buffer.data());
  g_encoded_tensor_large.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  len = benchmark__inference_response__pack(g_inference, g_encode_buffer.data());
  g_encoded_inference.assign(g_encode_buffer.begin(), g_encode_buffer.begin() + len);

  g_initialized = true;
}

static void BM_Protobuf_Encode_PersonSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__person__pack(g_small_person, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_small.size());
}

static void BM_Protobuf_Encode_PersonMedium(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__person__pack(g_medium_person, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_medium.size());
}

static void BM_Protobuf_Encode_OrderSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__order__pack(g_small_order, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_small.size());
}

static void BM_Protobuf_Encode_OrderLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__order__pack(g_large_order, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_large.size());
}

static void BM_Protobuf_Encode_EventSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__event__pack(g_small_event, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_small.size());
}

static void BM_Protobuf_Encode_EventLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__event__pack(g_large_event, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_large.size());
}

static void BM_Protobuf_Encode_TreeWide(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__tree_node__pack(g_wide_tree, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_wide.size());
}

static void BM_Protobuf_Encode_TreeDeep(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__tree_node__pack(g_deep_tree, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_deep.size());
}

static void BM_Protobuf_Encode_JsonSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__json_value__pack(g_small_json, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_small.size());
}

static void BM_Protobuf_Encode_JsonLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__json_value__pack(g_large_json, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_large.size());
}

static void BM_Protobuf_Encode_DocumentSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__document__pack(g_small_document, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_small.size());
}

static void BM_Protobuf_Encode_DocumentLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__document__pack(g_large_document, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_large.size());
}

static void BM_Protobuf_Encode_ChunkedText(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__chunked_text__pack(g_alice_chunks, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_alice.size());
}

static void BM_Protobuf_Decode_PersonSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__person__unpack(
        nullptr, g_encoded_person_small.size(), g_encoded_person_small.data()
    );
    benchmark::DoNotOptimize(msg->id);
    benchmark__person__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_small.size());
}

static void BM_Protobuf_Decode_PersonMedium(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__person__unpack(
        nullptr, g_encoded_person_medium.size(), g_encoded_person_medium.data()
    );
    benchmark::DoNotOptimize(msg->id);
    benchmark__person__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_person_medium.size());
}

static void BM_Protobuf_Decode_OrderSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__order__unpack(
        nullptr, g_encoded_order_small.size(), g_encoded_order_small.data()
    );
    benchmark::DoNotOptimize(msg->order_id);
    benchmark__order__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_small.size());
}

static void BM_Protobuf_Decode_OrderLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__order__unpack(
        nullptr, g_encoded_order_large.size(), g_encoded_order_large.data()
    );
    benchmark::DoNotOptimize(msg->order_id);
    benchmark__order__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_order_large.size());
}

static void BM_Protobuf_Decode_EventSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__event__unpack(
        nullptr, g_encoded_event_small.size(), g_encoded_event_small.data()
    );
    benchmark::DoNotOptimize(msg->id);
    benchmark__event__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_small.size());
}

static void BM_Protobuf_Decode_EventLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__event__unpack(
        nullptr, g_encoded_event_large.size(), g_encoded_event_large.data()
    );
    benchmark::DoNotOptimize(msg->id);
    benchmark__event__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_event_large.size());
}

static void BM_Protobuf_Decode_TreeWide(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__tree_node__unpack(
        nullptr, g_encoded_tree_wide.size(), g_encoded_tree_wide.data()
    );
    benchmark::DoNotOptimize(msg->value);
    benchmark__tree_node__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_wide.size());
}

static void BM_Protobuf_Decode_TreeDeep(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__tree_node__unpack(
        nullptr, g_encoded_tree_deep.size(), g_encoded_tree_deep.data()
    );
    benchmark::DoNotOptimize(msg->value);
    benchmark__tree_node__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tree_deep.size());
}

static void BM_Protobuf_Decode_JsonSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__json_value__unpack(
        nullptr, g_encoded_json_small.size(), g_encoded_json_small.data()
    );
    benchmark::DoNotOptimize(msg->value_case);
    benchmark__json_value__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_small.size());
}

static void BM_Protobuf_Decode_JsonLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__json_value__unpack(
        nullptr, g_encoded_json_large.size(), g_encoded_json_large.data()
    );
    benchmark::DoNotOptimize(msg->value_case);
    benchmark__json_value__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_json_large.size());
}

static void BM_Protobuf_Decode_DocumentSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__document__unpack(
        nullptr, g_encoded_doc_small.size(), g_encoded_doc_small.data()
    );
    benchmark::DoNotOptimize(msg->title);
    benchmark__document__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_small.size());
}

static void BM_Protobuf_Decode_DocumentLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__document__unpack(
        nullptr, g_encoded_doc_large.size(), g_encoded_doc_large.data()
    );
    benchmark::DoNotOptimize(msg->title);
    benchmark__document__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_doc_large.size());
}

static void BM_Protobuf_Decode_ChunkedText(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg =
        benchmark__chunked_text__unpack(nullptr, g_encoded_alice.size(), g_encoded_alice.data());
    benchmark::DoNotOptimize(msg->n_spans);
    benchmark__chunked_text__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_alice.size());
}

static void BM_Protobuf_Roundtrip_PersonSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__person__pack(g_small_person, g_encode_buffer.data());
    auto* msg = benchmark__person__unpack(nullptr, len, g_encode_buffer.data());
    benchmark::DoNotOptimize(msg->id);
    benchmark__person__free_unpacked(msg, nullptr);
  }
}

static void BM_Protobuf_Roundtrip_OrderLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__order__pack(g_large_order, g_encode_buffer.data());
    auto* msg = benchmark__order__unpack(nullptr, len, g_encode_buffer.data());
    benchmark::DoNotOptimize(msg->order_id);
    benchmark__order__free_unpacked(msg, nullptr);
  }
}

static void BM_Protobuf_Roundtrip_EventLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__event__pack(g_large_event, g_encode_buffer.data());
    auto* msg = benchmark__event__unpack(nullptr, len, g_encode_buffer.data());
    benchmark::DoNotOptimize(msg->id);
    benchmark__event__free_unpacked(msg, nullptr);
  }
}

static void BM_Protobuf_Roundtrip_TreeDeep(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__tree_node__pack(g_deep_tree, g_encode_buffer.data());
    auto* msg = benchmark__tree_node__unpack(nullptr, len, g_encode_buffer.data());
    benchmark::DoNotOptimize(msg->value);
    benchmark__tree_node__free_unpacked(msg, nullptr);
  }
}

static void BM_Protobuf_Encode_Embedding768(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__embedding_bf16__pack(g_emb_768, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_768.size());
}

static void BM_Protobuf_Encode_Embedding1536(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__embedding_bf16__pack(g_emb_1536, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_1536.size());
}

static void BM_Protobuf_Encode_EmbeddingF32_768(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__embedding_f32__pack(g_emb_f32_768, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_f32_768.size());
}

static void BM_Protobuf_Encode_EmbeddingBatch(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__embedding_batch__pack(g_emb_batch, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_batch.size());
}

static void BM_Protobuf_Encode_LLMChunkSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__llmstream_chunk__pack(g_llm_small, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_llm_small.size());
}

static void BM_Protobuf_Encode_LLMChunkLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__llmstream_chunk__pack(g_llm_large, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_llm_large.size());
}

static void BM_Protobuf_Encode_TensorShardSmall(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__tensor_shard__pack(g_tensor_small, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tensor_small.size());
}

static void BM_Protobuf_Encode_TensorShardLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__tensor_shard__pack(g_tensor_large, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tensor_large.size());
}

static void BM_Protobuf_Encode_InferenceResponse(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    size_t len = benchmark__inference_response__pack(g_inference, g_encode_buffer.data());
    benchmark::DoNotOptimize(len);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_inference.size());
}

static void BM_Protobuf_Decode_Embedding768(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__embedding_bf16__unpack(
        nullptr, g_encoded_emb_768.size(), g_encoded_emb_768.data()
    );
    benchmark::DoNotOptimize(msg->vector.len);
    benchmark__embedding_bf16__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_768.size());
}

static void BM_Protobuf_Decode_Embedding1536(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__embedding_bf16__unpack(
        nullptr, g_encoded_emb_1536.size(), g_encoded_emb_1536.data()
    );
    benchmark::DoNotOptimize(msg->vector.len);
    benchmark__embedding_bf16__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_1536.size());
}

static void BM_Protobuf_Decode_EmbeddingBatch(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__embedding_batch__unpack(
        nullptr, g_encoded_emb_batch.size(), g_encoded_emb_batch.data()
    );
    benchmark::DoNotOptimize(msg->n_embeddings);
    benchmark__embedding_batch__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_emb_batch.size());
}

static void BM_Protobuf_Decode_LLMChunkLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__llmstream_chunk__unpack(
        nullptr, g_encoded_llm_large.size(), g_encoded_llm_large.data()
    );
    benchmark::DoNotOptimize(msg->n_tokens);
    benchmark__llmstream_chunk__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_llm_large.size());
}

static void BM_Protobuf_Decode_TensorShardLarge(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__tensor_shard__unpack(
        nullptr, g_encoded_tensor_large.size(), g_encoded_tensor_large.data()
    );
    benchmark::DoNotOptimize(msg->data.len);
    benchmark__tensor_shard__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_tensor_large.size());
}

static void BM_Protobuf_Decode_InferenceResponse(benchmark::State& state)
{
  init_protobuf_data();
  for (auto _ : state) {
    auto* msg = benchmark__inference_response__unpack(
        nullptr, g_encoded_inference.size(), g_encoded_inference.data()
    );
    benchmark::DoNotOptimize(msg->n_embeddings);
    benchmark__inference_response__free_unpacked(msg, nullptr);
  }
  state.SetBytesProcessed(state.iterations() * g_encoded_inference.size());
}

void RegisterProtobufBenchmarks()
{
  BENCHMARK(BM_Protobuf_Encode_PersonSmall);
  BENCHMARK(BM_Protobuf_Encode_PersonMedium);
  BENCHMARK(BM_Protobuf_Encode_OrderSmall);
  BENCHMARK(BM_Protobuf_Encode_OrderLarge);
  BENCHMARK(BM_Protobuf_Encode_EventSmall);
  BENCHMARK(BM_Protobuf_Encode_EventLarge);
  BENCHMARK(BM_Protobuf_Encode_TreeWide);
  BENCHMARK(BM_Protobuf_Encode_TreeDeep);

  BENCHMARK(BM_Protobuf_Decode_PersonSmall);
  BENCHMARK(BM_Protobuf_Decode_PersonMedium);
  BENCHMARK(BM_Protobuf_Decode_OrderSmall);
  BENCHMARK(BM_Protobuf_Decode_OrderLarge);
  BENCHMARK(BM_Protobuf_Decode_EventSmall);
  BENCHMARK(BM_Protobuf_Decode_EventLarge);
  BENCHMARK(BM_Protobuf_Decode_TreeWide);
  BENCHMARK(BM_Protobuf_Decode_TreeDeep);

  BENCHMARK(BM_Protobuf_Roundtrip_PersonSmall);
  BENCHMARK(BM_Protobuf_Roundtrip_OrderLarge);
  BENCHMARK(BM_Protobuf_Roundtrip_EventLarge);
  BENCHMARK(BM_Protobuf_Roundtrip_TreeDeep);

  BENCHMARK(BM_Protobuf_Encode_JsonSmall);
  BENCHMARK(BM_Protobuf_Encode_JsonLarge);
  BENCHMARK(BM_Protobuf_Decode_JsonSmall);
  BENCHMARK(BM_Protobuf_Decode_JsonLarge);

  BENCHMARK(BM_Protobuf_Encode_DocumentSmall);
  BENCHMARK(BM_Protobuf_Encode_DocumentLarge);
  BENCHMARK(BM_Protobuf_Decode_DocumentSmall);
  BENCHMARK(BM_Protobuf_Decode_DocumentLarge);

  BENCHMARK(BM_Protobuf_Encode_ChunkedText);
  BENCHMARK(BM_Protobuf_Decode_ChunkedText);

  BENCHMARK(BM_Protobuf_Encode_Embedding768);
  BENCHMARK(BM_Protobuf_Encode_Embedding1536);
  BENCHMARK(BM_Protobuf_Encode_EmbeddingF32_768);
  BENCHMARK(BM_Protobuf_Encode_EmbeddingBatch);
  BENCHMARK(BM_Protobuf_Encode_LLMChunkSmall);
  BENCHMARK(BM_Protobuf_Encode_LLMChunkLarge);
  BENCHMARK(BM_Protobuf_Encode_TensorShardSmall);
  BENCHMARK(BM_Protobuf_Encode_TensorShardLarge);
  BENCHMARK(BM_Protobuf_Encode_InferenceResponse);

  BENCHMARK(BM_Protobuf_Decode_Embedding768);
  BENCHMARK(BM_Protobuf_Decode_Embedding1536);
  BENCHMARK(BM_Protobuf_Decode_EmbeddingBatch);
  BENCHMARK(BM_Protobuf_Decode_LLMChunkLarge);
  BENCHMARK(BM_Protobuf_Decode_TensorShardLarge);
  BENCHMARK(BM_Protobuf_Decode_InferenceResponse);
}
