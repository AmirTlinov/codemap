use ctx_fixture_replay::seek_frame;

pub fn render_frame(time_ms: u64) -> String {
    format!("frame:{}", seek_frame(time_ms))
}
