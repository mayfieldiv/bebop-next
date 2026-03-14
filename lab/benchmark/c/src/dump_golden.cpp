#include "encode_helpers.h"
#include "bench_harness.h"

#include <cstdio>
#include <cstdlib>
#include <string>
#include <vector>

static int g_golden_count = 0;

static void write_golden(const char* dir, const char* name, const std::vector<uint8_t>& data)
{
  std::string path = std::string(dir) + "/" + name + ".bin";
  FILE* f = fopen(path.c_str(), "wb");
  if (!f) {
    fprintf(stderr, "ERROR: cannot open %s\n", path.c_str());
    exit(1);
  }
  fwrite(data.data(), 1, data.size(), f);
  fclose(f);
  g_golden_count++;
  printf("  %-30s %zu bytes\n", name, data.size());
}

int main(int argc, char* argv[])
{
  if (argc < 2) {
    fprintf(stderr, "Usage: dump_golden <output-dir>\n");
    return 1;
  }
  const char* dir = argv[1];

  printf("Dumping golden files to %s\n", dir);

  write_golden(dir, "PersonSmall", bebop_encode_person_once(GetSmallPerson()));
  write_golden(dir, "PersonMedium", bebop_encode_person_once(GetMediumPerson()));
  write_golden(dir, "OrderSmall", bebop_encode_order_once(GetSmallOrder()));
  write_golden(dir, "OrderLarge", bebop_encode_order_once(GetLargeOrder()));
  write_golden(dir, "EventSmall", bebop_encode_event_once(GetSmallEvent()));
  write_golden(dir, "EventLarge", bebop_encode_event_once(GetLargeEvent()));
  write_golden(dir, "TreeWide", bebop_encode_tree_wide_once());
  write_golden(dir, "TreeDeep", bebop_encode_tree_deep_once());
  write_golden(dir, "JsonSmall", bebop_encode_json_small_once());
  write_golden(dir, "JsonLarge", bebop_encode_json_large_once());
  write_golden(dir, "JsonNested", bebop_encode_json_nested_once());
  write_golden(dir, "DocumentSmall", bebop_encode_doc_small_once());
  write_golden(dir, "DocumentLarge", bebop_encode_doc_large_once());
  write_golden(dir, "ChunkedText", bebop_encode_chunked_text_once());
  write_golden(dir, "Embedding384", bebop_encode_embedding384_once());
  write_golden(dir, "Embedding768", bebop_encode_embedding768_once());
  write_golden(dir, "Embedding1536", bebop_encode_embedding1536_once());
  write_golden(dir, "EmbeddingF32_768", bebop_encode_embedding_f32_768_once());
  write_golden(dir, "EmbeddingBatch", bebop_encode_embedding_batch_once());
  write_golden(dir, "LLMChunkSmall", bebop_encode_llm_chunk_small_once());
  write_golden(dir, "LLMChunkLarge", bebop_encode_llm_chunk_large_once());
  write_golden(dir, "TensorShardSmall", bebop_encode_tensor_shard_small_once());
  write_golden(dir, "TensorShardLarge", bebop_encode_tensor_shard_large_once());
  write_golden(dir, "InferenceResponse", bebop_encode_inference_response_once());

  printf("Done — %d golden files written.\n", g_golden_count);
  return 0;
}
