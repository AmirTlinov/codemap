package session

import "example.com/ctx/replay/timeline"

func FrameLabel(frame int) string {
    selected := Seek([]timeline.Frame{{Number: frame}}, frame)
    return selected.Label
}
