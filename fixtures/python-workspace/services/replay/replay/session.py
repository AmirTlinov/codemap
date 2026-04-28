from .timeline import frame_at


def seek(frames: list[int], frame: int) -> int:
    return frame_at(frames, frame)


def frame_label(frame: int) -> str:
    return f"frame-{seek([frame], frame)}"
