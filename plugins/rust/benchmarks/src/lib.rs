pub mod benchmark_types;

use std::borrow::Cow;
use std::collections::HashMap;

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

  pub fn embedding_bf16_1536() -> bt::EmbeddingBf16 {
    let vector = (0..1536)
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

  pub fn inference_response() -> bt::InferenceResponse {
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
