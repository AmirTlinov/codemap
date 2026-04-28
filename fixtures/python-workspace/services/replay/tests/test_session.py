from replay.session import seek


def test_seek_uses_timeline_frame() -> None:
    assert seek([12], 12) == 12
