import PetaCore
@_exported import PetaShown
import PetaTypes

/// The parts of an item as it is shown, with the links to pages and tags that the core writes.
/// The page of a tag is under `tagsDir`.
public func shownBlocks(ast: Node, tagsDir: String) -> [ShownBlock] {
    shownBlocks(ast: ast, links: pageLinks(tagsDir: tagsDir))
}

/// The links to pages and tags as the core writes them: the URL of the page in the app, such
/// as `/pages/notes`.
func pageLinks(tagsDir: String) -> PageLinks {
    PageLinks(page: { pageUrl(page: $0) }, tag: { tagUrl(tagsDir: tagsDir, tag: $0) })
}
