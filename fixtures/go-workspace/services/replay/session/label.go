package session

import "example.com/codemap/replay/timeline"

func FrameLabel(frame int) string {
    selected := Seek([]timeline.Frame{{Number: frame}}, frame)
    return selected.Label
}
