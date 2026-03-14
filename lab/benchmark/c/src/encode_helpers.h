#pragma once

#include <cstdint>
#include <vector>

extern "C" {
#include "bebop_wire.h"
#include "benchmark.bb.h"
}

// Forward declarations for test fixture types
struct TestPerson;
struct TestOrder;
struct TestEvent;
struct TestTreeNode;
struct TestJsonValue;
struct TestDocument;
struct TestEmbeddingBF16;
struct TestEmbeddingF32;
struct TestLLMStreamChunk;
struct TestTensorShard;
struct TestInferenceResponse;

// Conversion functions: build C wire structs from test fixture data.
Person make_person(const TestPerson& p);
Order make_order(const TestOrder& o);
Event make_event(const TestEvent& e);

void convert_tree_recursive(
    const TestTreeNode& src,
    TreeNode& dst,
    std::vector<TreeNode>& nodes,
    std::vector<TreeNode_Array>& children
);

JsonValue convert_json_value(
    Bebop_WireCtx* ctx,
    const TestJsonValue& src,
    std::vector<JsonValue>& storage,
    std::vector<JsonValue_Array>& arrays,
    std::vector<Bebop_Map>& maps
);

Document make_document(
    Bebop_WireCtx* ctx,
    const TestDocument& d,
    std::vector<JsonValue>& storage,
    std::vector<JsonValue_Array>& arrays,
    std::vector<Bebop_Map>& maps
);

EmbeddingBF16 make_embedding_bf16(const TestEmbeddingBF16& e);
EmbeddingF32 make_embedding_f32(const TestEmbeddingF32& e);

// Encode-once helpers: each builds the C struct from test data, encodes it,
// and returns the raw wire bytes.
std::vector<uint8_t> bebop_encode_person_once(const TestPerson& p);
std::vector<uint8_t> bebop_encode_order_once(const TestOrder& o);
std::vector<uint8_t> bebop_encode_event_once(const TestEvent& e);
std::vector<uint8_t> bebop_encode_tree_wide_once();
std::vector<uint8_t> bebop_encode_tree_deep_once();
std::vector<uint8_t> bebop_encode_json_small_once();
std::vector<uint8_t> bebop_encode_json_large_once();
std::vector<uint8_t> bebop_encode_json_nested_once();
std::vector<uint8_t> bebop_encode_doc_small_once();
std::vector<uint8_t> bebop_encode_doc_large_once();
std::vector<uint8_t> bebop_encode_chunked_text_once();
std::vector<uint8_t> bebop_encode_embedding384_once();
std::vector<uint8_t> bebop_encode_embedding768_once();
std::vector<uint8_t> bebop_encode_embedding1536_once();
std::vector<uint8_t> bebop_encode_embedding_f32_768_once();
std::vector<uint8_t> bebop_encode_embedding_batch_once();
std::vector<uint8_t> bebop_encode_llm_chunk_small_once();
std::vector<uint8_t> bebop_encode_llm_chunk_large_once();
std::vector<uint8_t> bebop_encode_tensor_shard_small_once();
std::vector<uint8_t> bebop_encode_tensor_shard_large_once();
std::vector<uint8_t> bebop_encode_inference_response_once();
