/// Golden file cross-language verification tests.
///
/// These tests load binary files produced by the C encoder (dump_golden),
/// decode them in Rust, re-encode, and verify byte-level or semantic equality.
///
/// The GOLDEN_DIR env var must point to the directory containing `.bin` files.
/// When unset, all tests are skipped (so `cargo test` alone still passes).
use bebop_runtime::{BebopDecode, BebopEncode, BebopReader, BebopWriter};

use bebop_rust_benchmarks::benchmark_types as bt;

fn load_golden(scenario: &str) -> Option<Vec<u8>> {
  let dir = std::env::var("GOLDEN_DIR").ok()?;
  let path = format!("{}/{}.bin", dir, scenario);
  Some(std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e)))
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
  a.iter()
    .zip(b.iter())
    .position(|(x, y)| x != y)
    .or_else(|| {
      if a.len() != b.len() {
        Some(a.len().min(b.len()))
      } else {
        None
      }
    })
}

/// Byte-exact round-trip: decode C bytes -> re-encode -> assert identical.
macro_rules! golden_exact {
  ($name:ident, $scenario:expr, $ty:ty) => {
    #[test]
    fn $name() {
      let golden = match load_golden($scenario) {
        Some(g) => g,
        None => {
          eprintln!("SKIP {}: GOLDEN_DIR not set", $scenario);
          return;
        }
      };

      let mut reader = BebopReader::new(&golden);
      let decoded = <$ty>::decode(&mut reader).unwrap_or_else(|e| {
        panic!(
          "{}: decode failed: {:?} (len={})",
          $scenario,
          e,
          golden.len()
        )
      });

      let mut writer = BebopWriter::new();
      decoded.encode(&mut writer);
      let reencoded = writer.into_bytes();

      if golden != reencoded {
        let pos = first_diff(&golden, &reencoded).unwrap();
        let g = golden
          .get(pos)
          .map(|b| format!("0x{:02x}", b))
          .unwrap_or_else(|| "EOF".into());
        let r = reencoded
          .get(pos)
          .map(|b| format!("0x{:02x}", b))
          .unwrap_or_else(|| "EOF".into());
        panic!(
          "{}: byte mismatch at offset {} — golden={}, rust={} (golden len={}, rust len={})",
          $scenario,
          pos,
          g,
          r,
          golden.len(),
          reencoded.len()
        );
      }
    }
  };
}

/// Semantic round-trip for map-containing types: decode -> re-encode -> decode again,
/// then compare decoded values and sizes. Map iteration order may differ.
macro_rules! golden_semantic {
  ($name:ident, $scenario:expr, $ty:ty) => {
    #[test]
    fn $name() {
      let golden = match load_golden($scenario) {
        Some(g) => g,
        None => {
          eprintln!("SKIP {}: GOLDEN_DIR not set", $scenario);
          return;
        }
      };

      let mut reader = BebopReader::new(&golden);
      let decoded = <$ty>::decode(&mut reader).unwrap_or_else(|e| {
        panic!(
          "{}: decode failed: {:?} (len={})",
          $scenario,
          e,
          golden.len()
        )
      });

      let mut writer = BebopWriter::new();
      decoded.encode(&mut writer);
      let reencoded = writer.into_bytes();

      let mut reader2 = BebopReader::new(&reencoded);
      let decoded2 = <$ty>::decode(&mut reader2).unwrap_or_else(|e| {
        panic!(
          "{}: re-decode failed: {:?} (len={})",
          $scenario,
          e,
          reencoded.len()
        )
      });

      assert_eq!(
        decoded, decoded2,
        "{}: semantic mismatch after round-trip",
        $scenario
      );

      assert_eq!(
        golden.len(),
        reencoded.len(),
        "{}: size mismatch — golden={} bytes, rust={} bytes",
        $scenario,
        golden.len(),
        reencoded.len()
      );
    }
  };
}

// ---- Byte-exact scenarios (19) ----

golden_exact!(golden_person_small, "PersonSmall", bt::Person);
golden_exact!(golden_person_medium, "PersonMedium", bt::Person);
golden_exact!(golden_order_small, "OrderSmall", bt::Order);
golden_exact!(golden_order_large, "OrderLarge", bt::Order);
golden_exact!(golden_event_small, "EventSmall", bt::Event);
golden_exact!(golden_event_large, "EventLarge", bt::Event);
golden_exact!(golden_tree_wide, "TreeWide", bt::TreeNode);
golden_exact!(golden_tree_deep, "TreeDeep", bt::TreeNode);
golden_exact!(golden_chunked_text, "ChunkedText", bt::ChunkedText);
golden_exact!(golden_embedding_384, "Embedding384", bt::EmbeddingBf16);
golden_exact!(golden_embedding_768, "Embedding768", bt::EmbeddingBf16);
golden_exact!(golden_embedding_1536, "Embedding1536", bt::EmbeddingBf16);
golden_exact!(
  golden_embedding_f32_768,
  "EmbeddingF32_768",
  bt::EmbeddingF32
);
golden_exact!(golden_embedding_batch, "EmbeddingBatch", bt::EmbeddingBatch);
golden_exact!(golden_llm_chunk_small, "LLMChunkSmall", bt::LlmStreamChunk);
golden_exact!(golden_llm_chunk_large, "LLMChunkLarge", bt::LlmStreamChunk);
golden_exact!(
  golden_tensor_shard_small,
  "TensorShardSmall",
  bt::TensorShard
);
golden_exact!(
  golden_tensor_shard_large,
  "TensorShardLarge",
  bt::TensorShard
);
golden_exact!(
  golden_inference_response,
  "InferenceResponse",
  bt::InferenceResponse
);

// ---- Semantic scenarios (4, contain maps) ----

golden_semantic!(golden_json_small, "JsonSmall", bt::JsonValue);
golden_semantic!(golden_json_large, "JsonLarge", bt::JsonValue);
golden_semantic!(golden_document_small, "DocumentSmall", bt::Document);
golden_semantic!(golden_document_large, "DocumentLarge", bt::Document);
