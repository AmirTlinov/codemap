use ctx_fixture_renderer::render_frame;

pub fn app_tick(time_ms: u64) -> String {
    render_frame(time_ms)
}
