/// A point in time represented as seconds and nanoseconds since the Unix epoch.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BebopTimestamp {
  pub seconds: i64,
  pub nanos: i32,
}

/// A span of time represented as seconds and nanoseconds.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BebopDuration {
  pub seconds: i64,
  pub nanos: i32,
}

#[cfg(feature = "std")]
mod std_conversions {
  use super::*;
  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  impl From<SystemTime> for BebopTimestamp {
    fn from(t: SystemTime) -> Self {
      match t.duration_since(UNIX_EPOCH) {
        Ok(d) => BebopTimestamp {
          seconds: d.as_secs() as i64,
          nanos: d.subsec_nanos() as i32,
        },
        Err(e) => {
          let d = e.duration();
          BebopTimestamp {
            seconds: -(d.as_secs() as i64),
            nanos: -(d.subsec_nanos() as i32),
          }
        }
      }
    }
  }

  impl From<BebopTimestamp> for SystemTime {
    fn from(ts: BebopTimestamp) -> Self {
      let total_nanos = (ts.seconds as i128) * 1_000_000_000 + (ts.nanos as i128);
      if total_nanos >= 0 {
        let secs = (total_nanos / 1_000_000_000) as u64;
        let nanos = (total_nanos % 1_000_000_000) as u32;
        UNIX_EPOCH + Duration::new(secs, nanos)
      } else {
        let abs = total_nanos.unsigned_abs();
        let secs = (abs / 1_000_000_000) as u64;
        let nanos = (abs % 1_000_000_000) as u32;
        UNIX_EPOCH - Duration::new(secs, nanos)
      }
    }
  }

  impl From<Duration> for BebopDuration {
    fn from(d: Duration) -> Self {
      BebopDuration {
        seconds: d.as_secs() as i64,
        nanos: d.subsec_nanos() as i32,
      }
    }
  }

  impl From<BebopDuration> for Duration {
    /// # Panics
    /// Panics if the duration is negative, since `std::time::Duration` is unsigned.
    fn from(d: BebopDuration) -> Self {
      let total_nanos = (d.seconds as i128) * 1_000_000_000 + (d.nanos as i128);
      assert!(
        total_nanos >= 0,
        "cannot convert negative BebopDuration to std::time::Duration"
      );
      let secs = (total_nanos / 1_000_000_000) as u64;
      let nanos = (total_nanos % 1_000_000_000) as u32;
      Duration::new(secs, nanos)
    }
  }
}

#[cfg(feature = "chrono")]
mod chrono_conversions {
  use super::*;

  impl From<chrono::DateTime<chrono::Utc>> for BebopTimestamp {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
      BebopTimestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn timestamp_default_is_zero() {
    let ts = BebopTimestamp::default();
    assert_eq!(ts.seconds, 0);
    assert_eq!(ts.nanos, 0);
  }

  #[test]
  fn duration_default_is_zero() {
    let d = BebopDuration::default();
    assert_eq!(d.seconds, 0);
    assert_eq!(d.nanos, 0);
  }

  #[cfg(feature = "std")]
  mod std_tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn timestamp_eq_and_hash() {
      use core::hash::{Hash, Hasher};

      let a = BebopTimestamp {
        seconds: 1,
        nanos: 2,
      };
      let b = BebopTimestamp {
        seconds: 1,
        nanos: 2,
      };
      let c = BebopTimestamp {
        seconds: 1,
        nanos: 3,
      };
      assert_eq!(a, b);
      assert_ne!(a, c);

      let mut ha = std::hash::DefaultHasher::new();
      a.hash(&mut ha);
      let mut hb = std::hash::DefaultHasher::new();
      b.hash(&mut hb);
      assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn system_time_round_trip() {
      let now = UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);
      let ts = BebopTimestamp::from(now);
      assert_eq!(ts.seconds, 1_700_000_000);
      assert_eq!(ts.nanos, 123_456_789);
      let back: SystemTime = ts.into();
      assert_eq!(back, now);
    }

    #[test]
    fn system_time_before_epoch() {
      let before = UNIX_EPOCH - Duration::new(100, 500);
      let ts = BebopTimestamp::from(before);
      assert_eq!(ts.seconds, -100);
      assert_eq!(ts.nanos, -500);
      let back: SystemTime = ts.into();
      assert_eq!(back, before);
    }

    #[test]
    fn std_duration_round_trip() {
      let d = Duration::new(42, 999_999_999);
      let bd = BebopDuration::from(d);
      assert_eq!(bd.seconds, 42);
      assert_eq!(bd.nanos, 999_999_999);
      let back: Duration = bd.into();
      assert_eq!(back, d);
    }

    #[test]
    fn timestamp_mixed_sign_negative_seconds_positive_nanos() {
      // -1s + 500ms = -0.5s before epoch
      let ts = BebopTimestamp {
        seconds: -1,
        nanos: 500_000_000,
      };
      let st: SystemTime = ts.into();
      assert_eq!(st, UNIX_EPOCH - Duration::new(0, 500_000_000));
    }

    #[test]
    fn timestamp_mixed_sign_positive_seconds_negative_nanos() {
      // 1s - 500ms = +0.5s after epoch
      let ts = BebopTimestamp {
        seconds: 1,
        nanos: -500_000_000,
      };
      let st: SystemTime = ts.into();
      assert_eq!(st, UNIX_EPOCH + Duration::new(0, 500_000_000));
    }

    #[test]
    fn timestamp_non_normalized_nanos_overflow() {
      // 0s + 2B nanos = +2s
      let ts = BebopTimestamp {
        seconds: 0,
        nanos: 2_000_000_000,
      };
      let st: SystemTime = ts.into();
      assert_eq!(st, UNIX_EPOCH + Duration::new(2, 0));
    }

    #[test]
    fn timestamp_non_normalized_negative_nanos() {
      // 0s - 1.5B nanos = -1.5s
      let ts = BebopTimestamp {
        seconds: 0,
        nanos: -1_500_000_000,
      };
      let st: SystemTime = ts.into();
      assert_eq!(st, UNIX_EPOCH - Duration::new(1, 500_000_000));
    }

    #[test]
    fn timestamp_one_nanosecond_before_epoch() {
      // -1s + 999_999_999 nanos = -1 nanosecond before epoch
      let ts = BebopTimestamp {
        seconds: -1,
        nanos: 999_999_999,
      };
      let st: SystemTime = ts.into();
      assert_eq!(st, UNIX_EPOCH - Duration::new(0, 1));
    }

    #[test]
    fn duration_positive_seconds_negative_nanos() {
      // 1s - 500ms = 0.5s
      let bd = BebopDuration {
        seconds: 1,
        nanos: -500_000_000,
      };
      let d: Duration = bd.into();
      assert_eq!(d, Duration::new(0, 500_000_000));
    }

    #[test]
    fn duration_non_normalized_nanos_overflow() {
      // 0s + 2B nanos = 2s
      let bd = BebopDuration {
        seconds: 0,
        nanos: 2_000_000_000,
      };
      let d: Duration = bd.into();
      assert_eq!(d, Duration::new(2, 0));
    }

    #[test]
    #[should_panic(expected = "cannot convert negative BebopDuration")]
    fn negative_duration_panics() {
      let bd = BebopDuration {
        seconds: -1,
        nanos: 0,
      };
      let _: Duration = bd.into();
    }

    #[test]
    #[should_panic(expected = "cannot convert negative BebopDuration")]
    fn negative_nanos_only_duration_panics() {
      let bd = BebopDuration {
        seconds: 0,
        nanos: -1,
      };
      let _: Duration = bd.into();
    }
  }
}
