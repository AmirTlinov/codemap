use crate::timeline::frame_at;

pub fn seek_frame(time_ms: u64) -> u64 {
    frame_at(time_ms)
}
