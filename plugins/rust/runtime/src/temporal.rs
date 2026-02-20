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
      if ts.seconds >= 0 {
        UNIX_EPOCH + Duration::new(ts.seconds as u64, ts.nanos as u32)
      } else {
        UNIX_EPOCH - Duration::new((-ts.seconds) as u64, (-ts.nanos) as u32)
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
    /// Panics if the duration has negative seconds, since `std::time::Duration`
    /// is unsigned.
    fn from(d: BebopDuration) -> Self {
      assert!(
        d.seconds >= 0,
        "cannot convert negative BebopDuration to std::time::Duration"
      );
      Duration::new(d.seconds as u64, d.nanos as u32)
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
    #[should_panic(expected = "cannot convert negative BebopDuration")]
    fn negative_duration_panics() {
      let bd = BebopDuration {
        seconds: -1,
        nanos: 0,
      };
      let _: Duration = bd.into();
    }
  }
}
