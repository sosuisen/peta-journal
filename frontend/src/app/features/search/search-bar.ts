import { Component, computed, ElementRef, inject, signal, viewChild } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../core/services/api.service';
import { NavigationService } from '../../core/services/navigation.service';
import { SearchResult } from '../../core/models/search.model';
import { generateUntitledTitle } from '../../core/utils/title.utils';
import { classifyJournalFileName } from '../../core/utils/journal-filename.utils';
import { validateFileName } from '../../core/utils/filename-validation.utils';
import { isImeComposingKeydown } from '../../core/utils/keyboard.utils';

@Component({
  selector: 'app-search-bar',
  imports: [FormsModule],
  template: `
    <div class="search-container">
      <input
        #searchInput
        type="text"
        class="search-input"
        placeholder="Type Ctrl+/ to search"
        [ngModel]="query()"
        (ngModelChange)="onQueryChange($event)"
        (keydown)="onKeydown($event)"
        (focus)="showDropdown.set(true)"
        (blur)="onBlur()"
      />
      @if (showDropdown() && (pageResults().length > 0 || itemResults().length > 0 || showCreateButton())) {
        <div class="search-dropdown">
          @for (result of pageResults(); track result.pageId; let i = $index) {
            <div class="search-result" [class.selected]="i === selectedIndex()" (mousedown)="navigateToResult(result)">
              <div class="result-page">
                <svg class="result-icon" width="14" height="14"><use href="#icon-page"/></svg>
                <span class="page-title">{{ result.pageTitle }}</span>
              </div>
            </div>
          }
          @if (showCreateButton() && query().trim()) {
            <div class="search-create" [class.selected]="selectedIndex() === createButtonIndex()" (mousedown)="createPage()">
              + Create "{{ query() }}"
            </div>
          }
          @for (result of itemResults(); track result.pageId + result.itemId; let i = $index) {
            <div class="search-result" [class.selected]="itemResultOffset() + i === selectedIndex()" (mousedown)="navigateToResult(result)">
              <div class="result-item">
                <svg class="result-icon" width="14" height="14"><use href="#icon-item"/></svg>
                <div class="item-content">
                  <div class="item-page-path">{{ result.pageTitle }}</div>
                  <div class="item-snippet" [innerHTML]="result.snippet"></div>
                </div>
              </div>
            </div>
          }
        </div>
      }
    </div>
  `,
  styles: [`
    :host { flex: 1; max-width: 350px; }
    .search-container { position: relative; }
    .search-input {
      width: 100%;
      padding: 8px 12px;
      border: 1px solid #ddd;
      border-radius: 6px;
      font-size: 14px;
      outline: none;
      box-sizing: border-box;
    }
    .search-input:focus { border-color: #007bff; }
    .search-dropdown {
      position: absolute;
      top: 100%;
      left: 0;
      right: 0;
      background: white;
      border: 1px solid #ddd;
      border-radius: 6px;
      margin-top: 4px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.1);
      max-height: 300px;
      overflow-y: auto;
      z-index: 100;
    }
    .search-result {
      padding: 8px 12px;
      cursor: pointer;
    }
    .search-result:hover, .search-result.selected { background: #f5f5f5; }
    .result-page {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .result-icon {
      flex-shrink: 0;
      color: #888;
    }
    .page-title { font-weight: 500; }
    .result-item {
      display: flex;
      align-items: flex-start;
      gap: 8px;
    }
    .item-content {
      min-width: 0;
      flex: 1;
    }
    .item-page-path {
      font-size: 12px;
      color: #999;
    }
    .item-snippet {
      font-size: 12px;
      color: #666;
      margin-top: 2px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .item-snippet mark { background: #fff3b0; }
    .search-create {
      padding: 8px 12px;
      cursor: pointer;
      color: #007bff;
      border-top: 1px solid #eee;
    }
    .search-create:hover, .search-create.selected { background: #f5f5f5; }
  `]
})
export class SearchBarComponent {
  private readonly api = inject(ApiService);
  private readonly nav = inject(NavigationService);
  private readonly searchInput = viewChild<ElementRef<HTMLInputElement>>('searchInput');

  /** Exposed so parent can read the current query */
  readonly query = signal('');
  readonly pageResults = signal<SearchResult[]>([]);
  readonly itemResults = signal<SearchResult[]>([]);
  readonly showDropdown = signal(false);
  readonly showCreateButton = signal(false);
  readonly selectedIndex = signal(-1);

  /** Index of the create button in the flat navigation order */
  readonly createButtonIndex = computed(() => this.pageResults().length);
  /** Starting index of item results in the flat navigation order */
  readonly itemResultOffset = computed(() =>
    this.pageResults().length + (this.showCreateButton() && this.query().trim() ? 1 : 0)
  );

  private debounceTimer: ReturnType<typeof setTimeout> | null = null;

  /**
   * Handles search query input changes with debounce.
   * Fires both page-name and full-text searches in parallel, combining results (pages first).
   */
  onQueryChange(value: string): void {
    this.query.set(value);

    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    if (!value.trim()) {
      this.pageResults.set([]);
      this.itemResults.set([]);
      this.showCreateButton.set(false);
      return;
    }

    this.debounceTimer = setTimeout(() => this.executeSearch(value), 200);
  }

  private async executeSearch(value: string): Promise<void> {
    const resp = await this.api.searchCombined(value);

    const pageResults: SearchResult[] = [];
    const itemResults: SearchResult[] = [];
    for (const r of resp.results) {
      if (r.itemId) {
        itemResults.push(r);
      } else {
        pageResults.push(r);
      }
    }

    this.pageResults.set(pageResults);
    this.itemResults.set(itemResults);
    this.selectedIndex.set(-1);

    const hasTagTerms = value.trim().split(/\s+/).some(t => t.startsWith('#') && t.length > 1);
    const exactMatch = pageResults.some(r => r.pageTitle.toLowerCase() === value.toLowerCase());
    this.showCreateButton.set(!hasTagTerms && !exactMatch && value.trim().length > 0);
  }

  /**
   * Navigates to the page associated with the selected search result and resets the search state.
   */
  navigateToResult(result: SearchResult): void {
    this.clearSearch();
    this.nav.navigateToPage(result.pageTitle, false, false, false, result.itemId || undefined);
  }

  /** Empties the query, drops the results and closes the dropdown. */
  clearSearch(): void {
    this.showDropdown.set(false);
    this.query.set('');
    this.pageResults.set([]);
    this.itemResults.set([]);
    this.showCreateButton.set(false);
  }

  /**
   * Creates a new page using the current query as the title.
   */
  async createPage(): Promise<void> {
    let title = this.query().trim();

    if (!title) {
      title = generateUntitledTitle();
    }

    // Validate non-journal file names
    if (classifyJournalFileName(title) === 'none') {
      const stripped = title.replace(/\.md$/, '');
      const lastSlash = stripped.lastIndexOf('/');
      const fileName = lastSlash >= 0 ? stripped.substring(lastSlash + 1) : stripped;
      const error = validateFileName(fileName);
      if (error) { alert(error); return; }
    }

    this.clearSearch();
    this.nav.navigateToPage(title);
  }

  /**
   * Handles keyboard navigation within the search dropdown.
   */
  onKeydown(event: KeyboardEvent): void {
    if (isImeComposingKeydown(event)) return;

    const hasCreate = this.showCreateButton() && !!this.query().trim();
    const totalItems = this.pageResults().length + (hasCreate ? 1 : 0) + this.itemResults().length;
    const maxIndex = totalItems - 1;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      this.selectedIndex.update(i => i < maxIndex ? i + 1 : 0);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      this.selectedIndex.update(i => i > 0 ? i - 1 : maxIndex);
    } else if (event.key === 'Enter') {
      const idx = this.selectedIndex();
      if (idx < 0) return;
      event.preventDefault();
      const pageCount = this.pageResults().length;
      if (idx < pageCount) {
        this.navigateToResult(this.pageResults()[idx]);
      } else if (hasCreate && idx === pageCount) {
        this.createPage();
      } else {
        const itemIdx = idx - pageCount - (hasCreate ? 1 : 0);
        if (itemIdx >= 0 && itemIdx < this.itemResults().length) {
          this.navigateToResult(this.itemResults()[itemIdx]);
        }
      }
    } else if (event.key === 'Escape') {
      this.showDropdown.set(false);
      this.selectedIndex.set(-1);
    }
  }

  focusInput(): void {
    this.searchInput()?.nativeElement.focus();
  }

  onBlur(): void {
    setTimeout(() => this.showDropdown.set(false), 200);
  }
}
