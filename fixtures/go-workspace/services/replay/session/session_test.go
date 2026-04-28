package session

import "testing"

func TestSeekUsesTimelineFrame(t *testing.T) {
    got := Seek(nil, 12)
    if got.Number != 12 {
        t.Fatalf("expected frame 12, got %d", got.Number)
    }
}
