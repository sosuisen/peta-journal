import PetaCore

/// The place of a cursor in a text. The core counts characters (Unicode scalars), and UIKit
/// counts UTF-16 units. The core converts both ways, so that every shell converts alike (ADR 043).
public enum Cursor {
    /// A place inside a surrogate pair goes after its character.
    public static func characters(utf16 units: Int, in text: String) -> Int {
        Int(charsOfUtf16(text: text, units: UInt64(max(units, 0))))
    }

    /// A place past the end of the text is its end.
    public static func utf16(characters: Int, in text: String) -> Int {
        Int(utf16OfChars(text: text, chars: UInt64(max(characters, 0))))
    }
}
