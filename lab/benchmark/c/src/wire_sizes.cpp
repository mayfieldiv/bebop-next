#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

#include "bench_harness.h"

extern "C" {
#include "bebop_wire.h"
#include "benchmark.bb.h"
}

#ifdef BENCH_PROTOBUF
extern "C" {
#include "benchmark.pb-c.h"
}
#endif

#ifdef USE_ZSTD
#include <zstd.h>
#endif

#ifdef USE_LZ4
#include <lz4.h>
#endif

#ifdef USE_BROTLI
#include <brotli/encode.h>
#endif

#ifdef USE_SNAPPY
#include <snappy.h>
#endif

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

static void ensure_ctx()
{
  if (!g_ctx) {
    Bebop_WireCtxOpts opts = Bebop_WireCtx_DefaultOpts();
    opts.arena_options.allocator.alloc = libc_alloc;
    opts.arena_options.initial_block_size = 1024 * 1024;
    opts.initial_writer_size = 256 * 1024;
    g_ctx = Bebop_WireCtx_New(&opts);
    Bebop_WireCtx_WriterHint(g_ctx, 256 * 1024, &g_writer);
  }
}

struct WireSize {
  const char* name;
  const char* category;
  size_t bebop_size;
  size_t protobuf_size;
  size_t best_compressed_size;
  const char* best_compressor;
};

static std::vector<WireSize> g_sizes;

static size_t try_compress(const uint8_t* data, size_t len, const char** best_name)
{
  size_t best = len;
  *best_name = "none";

#ifdef USE_LZ4
  {
    int bound = LZ4_compressBound((int)len);
    std::vector<char> out(bound);
    int result = LZ4_compress_default((const char*)data, out.data(), (int)len, bound);
    if (result > 0 && (size_t)result < best) {
      best = (size_t)result;
      *best_name = "lz4";
    }
  }
#endif

#ifdef USE_ZSTD
  {
    size_t bound = ZSTD_compressBound(len);
    std::vector<uint8_t> out(bound);
    size_t result = ZSTD_compress(out.data(), bound, data, len, 1);
    if (!ZSTD_isError(result) && result < best) {
      best = result;
      *best_name = "zstd";
    }
  }
#endif

#ifdef USE_BROTLI
  {
    size_t out_size = BrotliEncoderMaxCompressedSize(len);
    std::vector<uint8_t> out(out_size);
    if (BrotliEncoderCompress(BROTLI_DEFAULT_QUALITY, BROTLI_DEFAULT_WINDOW,
                               BROTLI_MODE_GENERIC, len, data, &out_size, out.data())) {
      if (out_size < best) {
        best = out_size;
        *best_name = "brotli";
      }
    }
  }
#endif

#ifdef USE_SNAPPY
  {
    size_t out_size = snappy::MaxCompressedLength(len);
    std::vector<char> out(out_size);
    snappy::RawCompress((const char*)data, len, out.data(), &out_size);
    if (out_size < best) {
      best = out_size;
      *best_name = "snappy";
    }
  }
#endif

  return best;
}

static void get_writer_buf(uint8_t** buf, size_t* len)
{
  Bebop_Writer_Buf(g_writer, buf, len);
}

static void record(const char* cat, const char* name, size_t protobuf)
{
  uint8_t* buf;
  size_t bebop;
  get_writer_buf(&buf, &bebop);

  const char* compressor;
  size_t compressed = try_compress(buf, bebop, &compressor);

  g_sizes.push_back({name, cat, bebop, protobuf, compressed, compressor});
}

//
// Bebop encoders
//

static void bebop_person(const TestPerson& p)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Person person = {
      .name = {.data = p.name.c_str(), .length = (uint32_t)p.name.size()},
      .email = {.data = p.email.c_str(), .length = (uint32_t)p.email.size()},
      .id = p.id,
      .age = p.age};
  Person_Encode(g_writer, &person);
}

static void bebop_order(const TestOrder& o)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Order order = {
      .item_ids = {.data = const_cast<int64_t*>(o.item_ids.data()), .length = o.item_ids.size()},
      .quantities = {.data = const_cast<int32_t*>(o.quantities.data()), .length = o.quantities.size()},
      .order_id = o.order_id,
      .customer_id = o.customer_id,
      .total = o.total,
      .timestamp = o.timestamp};
  Order_Encode(g_writer, &order);
}

static void bebop_event(const TestEvent& e)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  Event event = {
      .payload = {.data = const_cast<uint8_t*>(e.payload.data()), .length = e.payload.size()},
      .type = {.data = e.type.c_str(), .length = (uint32_t)e.type.size()},
      .source = {.data = e.source.c_str(), .length = (uint32_t)e.source.size()},
      .id = e.id,
      .timestamp = e.timestamp};
  Event_Encode(g_writer, &event);
}

static void bebop_embedding_bf16(const TestEmbeddingBF16& e)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  EmbeddingBF16 emb = {
      .vector = {.data = (Bebop_BFloat16*)const_cast<uint16_t*>(e.vector.data()),
                 .length = e.vector.size()},
      .id = *reinterpret_cast<const Bebop_UUID*>(e.id.bytes)};
  EmbeddingBF16_Encode(g_writer, &emb);
}

static void bebop_tensor_shard(const TestTensorShard& t)
{
  ensure_ctx();
  Bebop_Writer_Reset(g_writer);
  TensorShard ts = {
      .name = {.data = t.name.c_str(), .length = (uint32_t)t.name.size()},
      .shape = {.data = const_cast<uint32_t*>(t.shape.data()), .length = t.shape.size()},
      .dtype = {.data = t.dtype.c_str(), .length = (uint32_t)t.dtype.size()},
      .data = {.data = (Bebop_BFloat16*)const_cast<uint16_t*>(t.data.data()), .length = t.data.size()},
      .offset = t.offset,
      .total_elements = t.total_elements};
  TensorShard_Encode(g_writer, &ts);
}

#ifdef BENCH_PROTOBUF
//
// Protobuf-c encoders
//

static size_t proto_person(const TestPerson& p)
{
  Benchmark__Person person = BENCHMARK__PERSON__INIT;
  person.id = p.id;
  person.name = const_cast<char*>(p.name.c_str());
  person.email = const_cast<char*>(p.email.c_str());
  person.age = p.age;
  return benchmark__person__get_packed_size(&person);
}

static size_t proto_order(const TestOrder& o)
{
  Benchmark__Order order = BENCHMARK__ORDER__INIT;
  order.order_id = o.order_id;
  order.customer_id = o.customer_id;
  order.n_item_ids = o.item_ids.size();
  order.item_ids = const_cast<int64_t*>(o.item_ids.data());
  order.n_quantities = o.quantities.size();
  order.quantities = const_cast<int32_t*>(o.quantities.data());
  order.total = o.total;
  order.timestamp = o.timestamp;
  return benchmark__order__get_packed_size(&order);
}

static size_t proto_event(const TestEvent& e)
{
  Benchmark__Event event = BENCHMARK__EVENT__INIT;
  event.id = e.id;
  event.type = const_cast<char*>(e.type.c_str());
  event.source = const_cast<char*>(e.source.c_str());
  event.timestamp = e.timestamp;
  event.payload.data = const_cast<uint8_t*>(e.payload.data());
  event.payload.len = e.payload.size();
  return benchmark__event__get_packed_size(&event);
}

static size_t proto_embedding_bf16(const TestEmbeddingBF16& e)
{
  Benchmark__EmbeddingBF16 emb = BENCHMARK__EMBEDDING_BF16__INIT;
  // id is a string in proto schema, encode uuid as hex or raw bytes
  static char id_str[33];
  snprintf(id_str, sizeof(id_str), "%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x",
           e.id.bytes[0], e.id.bytes[1], e.id.bytes[2], e.id.bytes[3],
           e.id.bytes[4], e.id.bytes[5], e.id.bytes[6], e.id.bytes[7],
           e.id.bytes[8], e.id.bytes[9], e.id.bytes[10], e.id.bytes[11],
           e.id.bytes[12], e.id.bytes[13], e.id.bytes[14], e.id.bytes[15]);
  emb.id = id_str;
  emb.vector.data = const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(e.vector.data()));
  emb.vector.len = e.vector.size() * sizeof(uint16_t);
  return benchmark__embedding_bf16__get_packed_size(&emb);
}

static size_t proto_tensor_shard(const TestTensorShard& t)
{
  Benchmark__TensorShard ts = BENCHMARK__TENSOR_SHARD__INIT;
  ts.name = const_cast<char*>(t.name.c_str());
  ts.n_shape = t.shape.size();
  ts.shape = const_cast<uint32_t*>(t.shape.data());
  ts.dtype = const_cast<char*>(t.dtype.c_str());
  ts.data.data = const_cast<uint8_t*>(reinterpret_cast<const uint8_t*>(t.data.data()));
  ts.data.len = t.data.size() * sizeof(uint16_t);
  ts.offset = t.offset;
  ts.total_elements = t.total_elements;
  return benchmark__tensor_shard__get_packed_size(&ts);
}
#endif

int main()
{
#ifdef BENCH_PROTOBUF
  // API Payloads
  bebop_person(GetSmallPerson());
  record("API", "PersonSmall", proto_person(GetSmallPerson()));
  bebop_person(GetMediumPerson());
  record("API", "PersonMedium", proto_person(GetMediumPerson()));
  bebop_order(GetSmallOrder());
  record("API", "OrderSmall", proto_order(GetSmallOrder()));
  bebop_order(GetLargeOrder());
  record("API", "OrderLarge", proto_order(GetLargeOrder()));

  // Event Telemetry
  bebop_event(GetSmallEvent());
  record("Event", "EventSmall", proto_event(GetSmallEvent()));
  bebop_event(GetLargeEvent());
  record("Event", "EventLarge", proto_event(GetLargeEvent()));

  // ML Inference
  bebop_embedding_bf16(GetEmbedding768());
  record("ML", "Embedding768", proto_embedding_bf16(GetEmbedding768()));
  bebop_embedding_bf16(GetEmbedding1536());
  record("ML", "Embedding1536", proto_embedding_bf16(GetEmbedding1536()));
  bebop_tensor_shard(GetTensorShardSmall());
  record("ML", "TensorShardSmall", proto_tensor_shard(GetTensorShardSmall()));
  bebop_tensor_shard(GetTensorShardLarge());
  record("ML", "TensorShardLarge", proto_tensor_shard(GetTensorShardLarge()));

  // Output JSON
  printf("{\n");
  printf("  \"wire_sizes\": [\n");
  for (size_t i = 0; i < g_sizes.size(); i++) {
    const auto& s = g_sizes[i];
    printf("    {\"category\": \"%s\", \"name\": \"%s\", \"bebop\": %zu, \"protobuf\": %zu, \"compressed\": %zu, \"compressor\": \"%s\"}%s\n",
           s.category, s.name, s.bebop_size, s.protobuf_size, s.best_compressed_size, s.best_compressor,
           i < g_sizes.size() - 1 ? "," : "");
  }
  printf("  ]\n");
  printf("}\n");
#else
  fprintf(stderr, "Error: Protobuf support not compiled in. Build with -DBENCH_PROTOBUF=ON\n");
  return 1;
#endif

  return 0;
}
