#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

struct TestPerson {
  int32_t id;
  std::string name;
  std::string email;
  int32_t age;
};

struct TestOrder {
  int64_t order_id;
  int64_t customer_id;
  std::vector<int64_t> item_ids;
  std::vector<int32_t> quantities;
  double total;
  int64_t timestamp;
};

struct TestEvent {
  int64_t id;
  std::string type;
  std::string source;
  int64_t timestamp;
  std::vector<uint8_t> payload;
};

struct TestTreeNode {
  int32_t value;
  std::vector<TestTreeNode> children;
};

struct TestJsonValue {
  enum class Type {
    Null,
    Bool,
    Number,
    String,
    List,
    Object
  };
  Type type;
  bool bool_val;
  double number_val;
  std::string string_val;
  std::vector<TestJsonValue> list_val;
  std::vector<std::pair<std::string, TestJsonValue>> object_val;
};

struct TestDocument {
  std::string title;
  std::string body;
  std::vector<std::pair<std::string, TestJsonValue>> metadata;
};

enum class TestChunkKind : uint8_t {
  Unknown = 0,
  Paragraph = 1,
  Chapter = 2,
  Heading = 3
};

struct TestTextSpan {
  uint32_t start;
  uint32_t len;
  TestChunkKind kind;
};

struct TestChunkedText {
  std::string source;
  std::vector<TestTextSpan> spans;
};

struct TestUUID {
  uint8_t bytes[16];
};

struct TestDuration {
  int64_t seconds;
  int32_t nanos;
};

struct TestEmbeddingBF16 {
  TestUUID id;
  std::vector<uint16_t> vector;
};

struct TestEmbeddingF32 {
  TestUUID id;
  std::vector<float> vector;
};

struct TestEmbeddingBatch {
  std::string model;
  std::vector<TestEmbeddingBF16> embeddings;
  uint32_t usage_tokens;
};

struct TestTokenLogprob {
  std::string token;
  uint32_t token_id;
  float logprob;
};

struct TestTokenAlternatives {
  std::vector<TestTokenLogprob> top_tokens;
};

struct TestLLMStreamChunk {
  uint32_t chunk_id;
  std::vector<std::string> tokens;
  std::vector<TestTokenAlternatives> logprobs;
  std::string finish_reason;
};

struct TestTensorShard {
  std::string name;
  std::vector<uint32_t> shape;
  std::string dtype;
  std::vector<uint16_t> data;
  uint64_t offset;
  uint64_t total_elements;
};

struct TestInferenceTiming {
  TestDuration queue_time;
  TestDuration inference_time;
  float tokens_per_second;
};

struct TestInferenceResponse {
  TestUUID request_id;
  std::vector<TestEmbeddingBF16> embeddings;
  TestInferenceTiming timing;
};

const TestPerson& GetSmallPerson();
const TestPerson& GetMediumPerson();
const TestOrder& GetSmallOrder();
const TestOrder& GetLargeOrder();
const TestEvent& GetSmallEvent();
const TestEvent& GetLargeEvent();
const TestTreeNode& GetWideTree();
const TestTreeNode& GetDeepTree();
const TestJsonValue& GetSmallJson();
const TestJsonValue& GetLargeJson();
const TestJsonValue& GetNestedJson();
const TestDocument& GetSmallDocument();
const TestDocument& GetLargeDocument();
const TestChunkedText& GetAliceChunks();

const TestEmbeddingBF16& GetEmbedding384();
const TestEmbeddingBF16& GetEmbedding768();
const TestEmbeddingBF16& GetEmbedding1536();
const TestEmbeddingF32& GetEmbeddingF32_768();
const TestEmbeddingBatch& GetEmbeddingBatch();
const TestLLMStreamChunk& GetLLMChunkSmall();
const TestLLMStreamChunk& GetLLMChunkLarge();
const TestTensorShard& GetTensorShardSmall();
const TestTensorShard& GetTensorShardLarge();
const TestInferenceResponse& GetInferenceResponse();
