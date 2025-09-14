// Tiny guard for OAP/1 streams: caps + idle/read timeouts.
// Integrates without touching existing modules.

#![forbid(unsafe_code)]

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct OapLimits {
    pub read_timeout: Duration,     // absolute cap for reading a whole stream
    pub idle_timeout: Duration,     // inactivity between frames
    pub max_frames_per_stream: u32, // frame count cap
    pub max_total_bytes_per_stream: u64, // payload cap across all DATA frames
}

impl Default for OapLimits {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(10),
            max_frames_per_stream: 4_096,
            max_total_bytes_per_stream: 64 * 1024 * 1024, // 64 MiB
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    Timeout,                      // idle or read timeout
    TooManyFrames { limit: u32 }, // exceeded frame cap
    TooManyBytes { limit: u64 },  // exceeded byte cap
}

#[derive(Clone, Debug)]
pub struct StreamState {
    started_at: Instant,
    last_activity: Instant,
    frames_seen: u32,
    bytes_seen: u64,
}

impl StreamState {
    pub fn new(now: Instant) -> Self {
        Self {
            started_at: now,
            last_activity: now,
            frames_seen: 0,
            bytes_seen: 0,
        }
    }

    #[inline]
    pub fn touch(&mut self, now: Instant) {
        self.last_activity = now;
    }

    /// Call on every DATA frame before accepting it.
    pub fn on_frame(
        &mut self,
        data_len: usize,
        now: Instant,
        lim: &OapLimits,
    ) -> Result<(), RejectReason> {
        // Check absolute read timeout (since stream start)
        if now.duration_since(self.started_at) > lim.read_timeout {
            return Err(RejectReason::Timeout);
        }
        // Check idle timeout (since last activity)
        if now.duration_since(self.last_activity) > lim.idle_timeout {
            return Err(RejectReason::Timeout);
        }
        // Check frame count
        let next_frames = self.frames_seen.saturating_add(1);
        if next_frames > lim.max_frames_per_stream {
            return Err(RejectReason::TooManyFrames {
                limit: lim.max_frames_per_stream,
            });
        }
        // Check byte budget
        let next_bytes = self.bytes_seen.saturating_add(data_len as u64);
        if next_bytes > lim.max_total_bytes_per_stream {
            return Err(RejectReason::TooManyBytes {
                limit: lim.max_total_bytes_per_stream,
            });
        }

        // Accept
        self.frames_seen = next_frames;
        self.bytes_seen = next_bytes;
        self.last_activity = now;
        Ok(())
    }
}
