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
    /// How much larger than the usual text a heading of each level is, from the first to the
    /// fifth. The web frontend has the same sizes.
    public static let headingScales: [Double] = [1.6, 1.4, 1.2, 1.1, 1]

    /// The level of the heading when the whole body is one heading line, and 0 otherwise. Such
    /// an item is edited at the size of its heading, in bold: the text does not jump when the
    /// cursor goes in.
    public var editedHeading: Int {
        headingLine ? Int(headingLevel) : 0
    }
}
