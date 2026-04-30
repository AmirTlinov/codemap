import Foundation
import SwiftUI

@MainActor
public final class ReplayViewModel: ObservableObject {
    public struct NavigationFrame {
        let label: String
    }

    public var title: String {
        "Replay"
    }

    private let frames: [NavigationFrame] = []

    public func seekFrame(_ index: Int) -> NavigationFrame? {
        frames.indices.contains(index) ? frames[index] : nil
    }
}

private enum ReplayMode {
    case paused
    case playing
}
