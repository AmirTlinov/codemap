package timeline

import "fmt"

type Frame struct {
    Number int
    Label  string
}

func At(frames []Frame, frame int) Frame {
    for _, candidate := range frames {
        if candidate.Number == frame {
            return candidate
        }
    }
    return Frame{Number: frame, Label: fmt.Sprintf("frame-%d", frame)}
}
