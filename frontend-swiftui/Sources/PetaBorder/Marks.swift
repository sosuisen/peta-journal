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
