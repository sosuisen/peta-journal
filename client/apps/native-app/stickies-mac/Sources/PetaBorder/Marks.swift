import PetaTypes

extension Row {
    /// What stands before the text of the item: the bullet, and after it the state of a task.
    /// Every item has a bullet, a task as well. That is so in all shells: the bullet is what
    /// makes the item a part of the outline, and the state of the task is said on top of it.
    public var marks: [String] {
        // A circle around the bullet says that children are hidden, as in the web frontend
        let bullet = isCollapsed ? "◉" : "•"
        guard let task else { return [bullet] }
        // Not a switch: the optimizing compiler of Swift 6.4.2 crashes on one over this enum
        if task == .done { return [bullet, "☑"] }
        // The box with a line is the one all shells mean for a running task: the checkbox of
        // the web that is neither checked nor empty
        return [bullet, task == .running ? "⊟" : "☐"]
    }
}

extension Row {
    /// How much larger than the usual text a heading of each level is, from the first to the
    /// fifth. The web frontend has the same sizes.
    public static let headingScales: [Double] = [1.6, 1.4, 1.2, 1.1, 1]

    /// The level of the heading when the whole body is one heading line, and 0 otherwise. Such
    /// an item is edited at the size of its heading, in bold: the text does not jump when the
    /// cursor goes in.
    public var editedHeading: Int {
        headingLine ? Int(headingLevel) : 0
    }

    /// How much larger than the usual text the first line of the item is drawn. The marks
    /// before the text stand in the middle of that line. An item that is edited shows its
    /// Markdown, which is large only when it is one heading line.
    public var firstLineScale: Double {
        let level = isEditing ? editedHeading : Int(headingLevel)
        return (1...5).contains(level) ? Row.headingScales[level - 1] : 1
    }
}
