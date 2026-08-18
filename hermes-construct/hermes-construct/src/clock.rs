use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// A robust timestamp representing time within the Fabric.
/// It tracks both wall clock time and logical progression if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FabricTimestamp {
    pub seconds: u64,
    pub nanos: u32,
}

impl FabricTimestamp {
    /// Creates a timestamp from the current system time.
    pub fn now() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self {
            seconds: now.as_secs(),
            nanos: now.subsec_nanos(),
        }
    }

    /// Creates a timestamp from specific components.
    pub fn new(seconds: u64, nanos: u32) -> Self {
        Self { seconds, nanos }
    }
}

/// The Clock module manages temporal progression and provides synchronized timing
/// across the kernel modules.
pub struct Clock {
    start_time: FabricTimestamp,
    tick_rate: Duration,
}

impl Clock {
    pub fn new(tick_rate: Duration) -> Self {
        Self {
            start_time: FabricTimestamp::now(),
            tick_rate,
        }
    }

    pub fn current_time(&self) -> FabricTimestamp {
        FabricTimestamp::now()
    }

    pub fn elapsed(&self) -> Duration {
        let now = FabricTimestamp::now();
        Duration::new(now.seconds - self.start_time.seconds, now.nanos.saturating_sub(self.start_time.nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_fabric_timestamp_now() {
        let t1 = FabricTimestamp::now();
        sleep(Duration::from_millis(2));
        let t2 = FabricTimestamp::now();
        assert!(t2 > t1);
    }

    #[test]
    fn test_clock_elapsed() {
        let clock = Clock::new(Duration::from_millis(10));
        sleep(Duration::from_millis(50));
        let elapsed = clock.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
    }
}
