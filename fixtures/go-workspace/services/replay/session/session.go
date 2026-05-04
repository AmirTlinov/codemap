package session

import "example.com/codemap/replay/timeline"

func Seek(frames []timeline.Frame, frame int) timeline.Frame {
    return timeline.At(frames, frame)
}
