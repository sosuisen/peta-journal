import PetaTypes

extension Row {
    /// What stands before the text of the item: the bullet, and after it the state of a task.
    /// A task has a bullet as well: the state of the task is said on top of it. Whether the
    /// bullet is seen is `showsBullet(bare:)`.
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

    /// The bullet before the text is seen. The core shows none before an empty item that is
    /// not edited, as in the other shells (#1042). A `bare` sticky shows only its text.
    public func showsBullet(bare: Bool) -> Bool {
        showBullet && !bare
    }
}

extension Row {
    /// The level of the heading when the whole body is one heading line, and 0 otherwise. Such
    /// an item is edited at the size of its heading, in bold: the text does not jump when the
    /// cursor goes in.
    public var editedHeading: Int {
        headingLine ? Int(headingLevel) : 0
    }

    /// How much larger than the usual text the Markdown in the editor is. It is large only when
    /// the whole body is one heading line, and then it is the scale of the first line that the
    /// core gives, edited or not: the cursor may go in before the core counts the item as
    /// edited (#795).
    public var editedScale: Double {
        headingLine ? Double(firstLineScale) : 1
    }
}
