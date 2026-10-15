/**
 * What the outline component does for the services that change items: the things that
 * need the rendered page. Services receive it per call, so tests pass a fake.
 */
export interface OutlineView {
  /** The page the outline shows. */
  pageId(): string;
  /** Replaces all items with the server's version of the page, e.g. after a transclusion changed. */
  reloadPage(): Promise<void>;
  /** Replaces the items of one page with the server's version, e.g. after a save failed. */
  reloadPageFromFile(pageId: string): void;
  /** Focuses the editor of an item after the next render; the first editor on the page when omitted. */
  focusEditor(itemId?: string): Promise<void>;
}
