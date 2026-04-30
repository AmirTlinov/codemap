import SwiftFixture
import Testing

@Test
func seekFrameKeepsReplayNavigationBounded() {
    let model = ReplayViewModel()
    #expect(model.seekFrame(0) == nil)
}
