import { Component, OnDestroy, output, signal } from '@angular/core';
import { ApiService } from '../../core/services/api.service';
import { Item } from '../../core/models/item.model';
import { WikiLinkContext } from './item-editor';

/** The page the user picked for the `[[wiki link]]` being typed. */
export interface WikiSuggestionPick {
  /** The item whose editor the link is typed in. */
  itemId: string;
  title: string;
}

/**
 * The popup with page names that match the text typed inside a `[[wiki link]]`.
 * The outline feeds it the query from the editor and inserts the pick.
 */
@Component({
  selector: 'app-wiki-suggestions',
  template: `
    @if (suggestions().length > 0) {
      <div
        class="wiki-suggestions"
        [style.left.px]="position().x"
        [style.top.px]="position().y"
      >
        @for (suggestion of suggestions(); track suggestion; let i = $index) {
          <div
            class="wiki-suggestion-item"
            [class.selected]="i === index()"
            (mousedown)="pick(suggestion, $event)"
            (mouseenter)="index.set(i)"
          >
            {{ suggestion }}
          </div>
        }
      </div>
    }
  `,
  styles: [`
    .wiki-suggestions {
      position: fixed;
      background: #fff;
      border: 1px solid #ddd;
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      max-height: 200px;
      overflow-y: auto;
      z-index: 1000;
      min-width: 200px;
      max-width: 400px;
      padding: 4px 0;
    }
    .wiki-suggestion-item {
      padding: 6px 12px;
      cursor: pointer;
      font-size: 13px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .wiki-suggestion-item.selected {
      background: #e8f0fe;
    }
    .wiki-suggestion-item:hover {
      background: #f0f0f0;
    }
  `]
})
export class WikiSuggestionsComponent implements OnDestroy {
  /** How long typing may pause before the page names are searched. */
  static readonly SEARCH_DELAY_MS = 150;

  readonly suggestions = signal<string[]>([]);
  /** The highlighted suggestion. */
  readonly index = signal(0);
  /** The item whose editor the link is typed in. */
  readonly itemId = signal<string | null>(null);
  readonly position = signal({ x: 0, y: 0 });
  /** Emits when the user picks a suggestion with the mouse or Enter. */
  readonly picked = output<WikiSuggestionPick>();

  private searchTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(private readonly api: ApiService) {}

  /** True when the popup is open for this item. */
  isShownFor(itemId: string): boolean {
    return this.itemId() === itemId && this.suggestions().length > 0;
  }

  /**
   * Handles the user typing inside a `[[wiki link]]` in the item editor.
   * Positions the popup and performs a debounced search for matching page names.
   * @param item - The item being edited
   * @param ctx - The wiki link context with query text and caret position, or null to close the popup
   */
  query(item: Item, ctx: WikiLinkContext | null): void {
    if (!ctx) {
      this.close();
      return;
    }
    this.itemId.set(item.id);
    this.position.set({ x: ctx.rect.left, y: ctx.rect.bottom + 4 });

    if (!ctx.query) {
      this.suggestions.set([]);
      return;
    }

    if (this.searchTimer) clearTimeout(this.searchTimer);
    this.searchTimer = setTimeout(async () => {
      try {
        const result = await this.api.searchPageNames(ctx.query, 10);
        this.suggestions.set(result.results.map(r => r.pageTitle));
        this.index.set(0);
      } catch (e) {
        this.suggestions.set([]);
      }
    }, WikiSuggestionsComponent.SEARCH_DELAY_MS);
  }

  /**
   * Handles the user pressing arrow keys or Enter while the popup is open.
   * Moves the highlight or picks the highlighted suggestion.
   * @param key - The key pressed ('ArrowUp', 'ArrowDown', or 'Enter')
   */
  keyNav(key: string): void {
    const suggestions = this.suggestions();
    if (suggestions.length === 0) return;
    const idx = this.index();

    if (key === 'ArrowDown') {
      this.index.set((idx + 1) % suggestions.length);
    } else if (key === 'ArrowUp') {
      this.index.set((idx - 1 + suggestions.length) % suggestions.length);
    } else if (key === 'Enter') {
      this.pick(suggestions[idx]);
    }
  }

  /**
   * Picks a suggestion and closes the popup.
   * @param title - The page title picked
   * @param event - The mouse event of a click, which must not blur the editor
   */
  pick(title: string, event?: MouseEvent): void {
    if (event) {
      event.preventDefault();
      event.stopPropagation();
    }
    const itemId = this.itemId();
    if (!itemId) return;
    this.picked.emit({ itemId, title });
    this.close();
  }

  close(): void {
    this.suggestions.set([]);
    this.itemId.set(null);
    if (this.searchTimer) {
      clearTimeout(this.searchTimer);
      this.searchTimer = null;
    }
  }

  ngOnDestroy(): void {
    this.close();
  }
}
