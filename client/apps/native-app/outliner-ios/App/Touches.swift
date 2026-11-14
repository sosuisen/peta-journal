import SwiftUI
import UIKit

/// Where in the shown text of an item a touch is. Each paragraph and code block tells the item
/// where it stands and what letters it shows (`LetterTarget`); the item finds the letter under
/// the touch, and the core puts the cursor at its place in the Markdown (ADR 042).
///
/// SwiftUI does not say which letter of a `Text` is at a point. TextKit lays out the same
/// letters with the same fonts at the same width, and says it.
enum Touches {
    /// The name of the space of the item, in which the places of its texts are measured.
    static let space = "item"

    /// How many letters of the shown text of the item stand before the touch at `point`: the
    /// letters before the text that is touched, and those before the touch in it. A touch
    /// beside the texts goes to the nearest line of the text nearest to it. `nil` when the
    /// item shows no text, as an image alone.
    static func letter(at point: CGPoint, in targets: [LetterTarget]) -> Int? {
        let nearest = targets.min { distance(point, $0.frame) < distance(point, $1.frame) }
        guard let target = nearest else { return nil }
        let inside = CGPoint(x: point.x - target.frame.minX, y: point.y - target.frame.minY)
        return target.start + letter(at: inside, in: target.text, width: target.wraps ? target.frame.width : .greatestFiniteMagnitude)
    }

    /// How many letters of `text`, laid out `width` wide, stand before a touch at `point`. The
    /// letters are counted as the core counts them: by Unicode scalar.
    static func letter(at point: CGPoint, in text: NSAttributedString, width: CGFloat) -> Int {
        let storage = NSTextStorage(attributedString: text)
        let layout = NSLayoutManager()
        storage.addLayoutManager(layout)
        let container = NSTextContainer(size: CGSize(width: max(width, 1), height: .greatestFiniteMagnitude))
        container.lineFragmentPadding = 0
        layout.addTextContainer(container)
        var fraction: CGFloat = 0
        let index = layout.characterIndex(for: point, in: container, fractionOfDistanceBetweenInsertionPoints: &fraction)
        let string = text.string
        // A touch on the right half of a letter is after it. A letter may be more than one
        // unit of UTF-16, as an emoji, and the touch is never between its units
        let units = fraction > 0.5 && index < text.length
            ? (string as NSString).rangeOfComposedCharacterSequence(at: index).upperBound
            : index
        let end = String.Index(utf16Offset: units, in: string)
        return string[..<end].unicodeScalars.count
    }

    /// How far a point is from a frame: 0 inside it. Up and down count more than sideways, so
    /// that a touch beside a line goes to that line.
    private static func distance(_ point: CGPoint, _ frame: CGRect) -> CGFloat {
        let across = max(frame.minX - point.x, 0, point.x - frame.maxX)
        let down = max(frame.minY - point.y, 0, point.y - frame.maxY)
        return across + down * 100
    }
}

/// A text of an item that a touch can fall on: where it stands in the item, and its letters.
struct LetterTarget: Equatable {
    /// How many letters of the shown text of the item stand before it, as the core counts.
    let start: Int
    let frame: CGRect
    /// The letters with the fonts that they are drawn with.
    let text: NSAttributedString
    /// False for the lines of code, which are never broken.
    let wraps: Bool
}

struct LetterTargets: PreferenceKey {
    static var defaultValue: [LetterTarget] { [] }

    static func reduce(value: inout [LetterTarget], nextValue: () -> [LetterTarget]) {
        value += nextValue()
    }
}

extension View {
    /// Tells the item where this text stands, so that a touch on it finds its letter.
    func letters(_ text: NSAttributedString, start: UInt32, wraps: Bool = true) -> some View {
        background(
            GeometryReader { place in
                Color.clear.preference(
                    key: LetterTargets.self,
                    value: [LetterTarget(start: Int(start), frame: place.frame(in: .named(Touches.space)), text: text, wraps: wraps)])
            })
    }
}
