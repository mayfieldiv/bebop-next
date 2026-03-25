#[rustfmt::skip]
pub mod benchmark_types;

use bebop_runtime::HashMap;
use std::borrow::Cow;

use bebop_runtime::{bf16, BebopDuration, Uuid};

use crate::benchmark_types as bt;

const ALICE_TEXT: &str =
  include_str!("../../../../lab/benchmark/fixtures/Alice's Adventures in Wonderland.md");

pub mod fixtures {
  use super::*;

  pub fn person_small() -> bt::Person<'static> {
    bt::Person::new(1, "Alice", "alice@example.com", 31)
  }

  pub fn person_medium() -> bt::Person<'static> {
    bt::Person::new(
      42,
      "Alice Bob Carol",
      "alice.bob.carol.longer-email@example.com",
      37,
    )
  }

  pub fn text_span() -> bt::TextSpan {
    bt::TextSpan::new(1024, 256, bt::ChunkKind::Paragraph)
  }

  pub fn embedding_bf16(n: usize) -> bt::EmbeddingBf16Owned {
    let vector = (0..n)
      .map(|i| bf16::from_f32((i as f32) * 0.001))
      .collect::<Vec<_>>();
    bt::EmbeddingBf16::new(
      Uuid::from_bytes([
        0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
      ]),
      vector,
    )
  }

  pub fn tree_deep(depth: usize) -> bt::TreeNode {
    if depth == 0 {
      return bt::TreeNode {
        value: Some(0),
        children: Some(Vec::new()),
      };
    }
    bt::TreeNode {
      value: Some(depth as i32),
      children: Some(vec![tree_deep(depth - 1)]),
    }
  }

  pub fn order_small() -> bt::OrderOwned {
    bt::Order::new(
      12345,
      67890,
      vec![101, 102, 103],
      vec![1, 2, 1],
      299.99,
      1704067200000,
    )
  }

  pub fn order_large() -> bt::OrderOwned {
    let item_ids: Vec<i64> = (1000..1100).collect();
    let quantities: Vec<i32> = (0..100).map(|i| (i % 10) + 1).collect();
    let total: f64 = quantities.iter().map(|&q| q as f64 * 49.99).sum();
    bt::Order::new(99999, 88888, item_ids, quantities, total, 1704067200000)
  }

  pub fn event_small() -> bt::Event<'static> {
    bt::Event::new(
      1001,
      "click",
      "web",
      1704067200000,
      vec![0x01, 0x02, 0x03, 0x04],
    )
  }

  pub fn event_large() -> bt::Event<'static> {
    let payload: Vec<u8> = (0..4096).map(|i| (i & 0xFF) as u8).collect();
    bt::Event::new(
      2002,
      "data_transfer",
      "backend_service_cluster_node_42",
      1704067200000,
      payload,
    )
  }

  pub fn tree_wide(children: usize) -> bt::TreeNode {
    let kids = (0..children)
      .map(|i| bt::TreeNode {
        value: Some(i as i32),
        children: Some(Vec::new()),
      })
      .collect();
    bt::TreeNode {
      value: Some(999),
      children: Some(kids),
    }
  }

  pub fn json_object_small() -> bt::JsonValue<'static> {
    let mut fields = HashMap::new();
    fields.insert(
      Cow::Borrowed("name"),
      bt::JsonValue::String(bt::String {
        value: Some(Cow::Borrowed("John Doe")),
      }),
    );
    fields.insert(
      Cow::Borrowed("age"),
      bt::JsonValue::Number(bt::Number { value: Some(30.0) }),
    );
    fields.insert(
      Cow::Borrowed("active"),
      bt::JsonValue::Bool(bt::Bool { value: Some(true) }),
    );
    bt::JsonValue::Object(bt::Object {
      fields: Some(fields),
    })
  }

  pub fn chunked_text() -> bt::ChunkedText<'static> {
    let source = ALICE_TEXT;
    let mut spans = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    while pos < len {
      // skip blank lines
      while pos < len && (bytes[pos] == b'\n' || bytes[pos] == b'\r') {
        pos += 1;
      }
      if pos >= len {
        break;
      }

      let start = pos;
      let kind = if pos + 3 < len
        && bytes[pos] == b'#'
        && bytes[pos + 1] == b'#'
        && bytes[pos + 2] == b' '
      {
        // heading line
        while pos < len && bytes[pos] != b'\n' {
          pos += 1;
        }
        bt::ChunkKind::Heading
      } else {
        // paragraph: read until double newline
        while pos < len {
          if bytes[pos] == b'\n' && pos + 1 < len && bytes[pos + 1] == b'\n' {
            break;
          }
          pos += 1;
        }
        bt::ChunkKind::Paragraph
      };

      if pos > start {
        spans.push(bt::TextSpan::new(start as u32, (pos - start) as u32, kind));
      }
    }

    bt::ChunkedText::new(source, spans)
  }

  pub fn embedding_f32(n: usize) -> bt::EmbeddingF32Owned {
    let vector = (0..n).map(|i| (i as f32) * 0.001).collect::<Vec<_>>();
    bt::EmbeddingF32::new(
      Uuid::from_bytes([
        0x30, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
      ]),
      vector,
    )
  }

  pub fn embedding_batch() -> bt::EmbeddingBatch<'static> {
    let embeddings = (0..8)
      .map(|i| {
        let vector = (0..1536)
          .map(|j| bf16::from_f32(((i * 1536 + j) as f32) * 0.0001))
          .collect::<Vec<_>>();
        bt::EmbeddingBf16::new(
          Uuid::from_bytes([
            0x40, i as u8, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
          ]),
          vector,
        )
      })
      .collect::<Vec<_>>();
    bt::EmbeddingBatch::new("text-embedding-3-small", embeddings, 256)
  }

  pub fn llm_chunk_small() -> bt::LlmStreamChunk<'static> {
    let tokens = vec!["Hello".to_string(), ",".to_string(), " world".to_string()];
    let logprobs: Vec<_> = tokens
      .iter()
      .map(|tok| bt::TokenAlternatives::new([bt::TokenLogprob::new(tok.clone(), 1000, -0.1)]))
      .collect();
    bt::LlmStreamChunk::new(1, tokens, logprobs, "")
  }

  pub fn llm_chunk_large() -> bt::LlmStreamChunk<'static> {
    let tokens: Vec<String> = (0..32).map(|i| format!("tk{i:02}")).collect();
    let logprobs: Vec<_> = (0..32)
      .map(|i| {
        let alts: Vec<_> = (0..5)
          .map(|k| bt::TokenLogprob::new(format!("a{i}_{k}"), (i * 5 + k) as u32, -0.5))
          .collect();
        bt::TokenAlternatives::new(alts)
      })
      .collect();
    bt::LlmStreamChunk::new(42, tokens, logprobs, "stop")
  }

  pub fn tensor_shard_small() -> bt::TensorShard<'static> {
    let data = (0..1024)
      .map(|i| bf16::from_f32((i as f32) * 0.001))
      .collect::<Vec<_>>();
    bt::TensorShard::new(
      "model.layers.0.attention.wq.weight",
      vec![4096, 4096],
      "bfloat16",
      data,
      0,
      4096 * 4096,
    )
  }

  pub fn json_small() -> bt::JsonValue<'static> {
    bt::JsonValue::String(bt::String {
      value: Some(Cow::Borrowed("hello")),
    })
  }

  pub fn json_large(depth: usize, width: usize) -> bt::JsonValue<'static> {
    if depth == 0 {
      return bt::JsonValue::Number(bt::Number { value: Some(42.5) });
    }

    let mut fields = HashMap::new();
    for i in 0..width {
      let key = Cow::Owned(format!("k{}", i));
      let value = if i % 2 == 0 {
        json_large(depth - 1, width)
      } else {
        bt::JsonValue::List(bt::List {
          values: Some(vec![json_large(depth - 1, width), json_small()]),
        })
      };
      fields.insert(key, value);
    }

    bt::JsonValue::Object(bt::Object {
      fields: Some(fields),
    })
  }

  pub fn document_small() -> bt::Document<'static> {
    let mut metadata = HashMap::new();
    metadata.insert(
      Cow::Borrowed("source"),
      bt::JsonValue::String(bt::String {
        value: Some(Cow::Borrowed("unit-test")),
      }),
    );

    bt::Document {
      title: Some(Cow::Borrowed("Short Doc")),
      body: Some(Cow::Borrowed("Hello world")),
      metadata: Some(metadata),
    }
  }

  pub fn document_large() -> bt::Document<'static> {
    let mut metadata = HashMap::new();
    metadata.insert(
      Cow::Borrowed("chapter_count"),
      bt::JsonValue::Number(bt::Number { value: Some(12.0) }),
    );
    metadata.insert(Cow::Borrowed("toc"), json_large(2, 4));

    bt::Document {
      title: Some(Cow::Borrowed("Alice's Adventures in Wonderland")),
      body: Some(Cow::Owned(ALICE_TEXT.to_string())),
      metadata: Some(metadata),
    }
  }

  pub fn tensor_shard_large() -> bt::TensorShard<'static> {
    let shape = vec![1, 40, 96, 96];
    let element_count = shape.iter().product::<u32>() as usize;
    let data = (0..element_count)
      .map(|i| bf16::from_f32(((i % 1024) as f32) * 0.0005))
      .collect::<Vec<_>>();

    bt::TensorShard::new(
      "encoder.layer.4.weight",
      shape,
      "bf16",
      data,
      0,
      element_count as u64,
    )
  }

  pub fn inference_response() -> bt::InferenceResponseOwned {
    let embeddings = (0..8)
      .map(|i| {
        let vector = (0..384)
          .map(|j| bf16::from_f32(((i * 384 + j) as f32) * 0.0001))
          .collect::<Vec<_>>();
        bt::EmbeddingBf16::new(
          Uuid::from_bytes([
            0x20, i as u8, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
          ]),
          vector,
        )
      })
      .collect::<Vec<_>>();

    let timing = bt::InferenceTiming::new(
      BebopDuration {
        seconds: 0,
        nanos: 8_000_000,
      },
      BebopDuration {
        seconds: 0,
        nanos: 72_000_000,
      },
      1234.56,
    );

    bt::InferenceResponse::new(
      Uuid::from_bytes([
        0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x10, 0x32, 0x54, 0x76, 0x98, 0xba, 0xdc, 0xfe, 0x11,
        0x22,
      ]),
      embeddings,
      timing,
    )
  }
}
