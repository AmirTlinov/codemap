use ctx_fixture_replay::seek_frame;

#[test]
fn seeks_to_deterministic_frame() {
    assert_eq!(seek_frame(32), 2);
}
