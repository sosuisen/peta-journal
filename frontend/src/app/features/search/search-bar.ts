import { Component, ElementRef, inject, signal, viewChild } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../core/services/api.service';

import { RecentPagesService } from '../../core/services/recent-pages.service';
import { NavigationService } from '../../core/services/navigation.service';
import { SearchResult } from '../../core/models/search.model';
import { generateUntitledTitle } from '../../core/utils/title.utils';
import { classifyJournalFileName } from '../../core/utils/journal-filename.utils';
import { validateFileName } from '../../core/utils/filename-validation.utils';

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
      @if (showDropdown() && (results().length > 0 || showCreateButton())) {
        <div class="search-dropdown">
          @for (result of results(); track result.pageId + result.blockId; let i = $index) {
            <div class="search-result" [class.selected]="i === selectedIndex()" (mousedown)="navigateToResult(result)">
              <div class="result-title">{{ result.pageTitle }}</div>
              @if (result.snippet) {
                <div class="result-snippet" [innerHTML]="result.snippet"></div>
              }
            </div>
          }
          @if (showCreateButton() && query().trim()) {
            <div class="search-create" [class.selected]="selectedIndex() === results().length" (mousedown)="createPage()">
              + Create "{{ query() }}"
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
    .result-title { font-weight: 500; }
    .result-snippet { font-size: 12px; color: #666; margin-top: 2px; }
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
  private readonly recentPages = inject(RecentPagesService);
  private readonly searchInput = viewChild<ElementRef<HTMLInputElement>>('searchInput');

  /** Exposed so parent can read the current query */
  readonly query = signal('');
  readonly results = signal<SearchResult[]>([]);
  readonly showDropdown = signal(false);
  readonly showCreateButton = signal(false);
  readonly selectedIndex = signal(-1);

  async onQueryChange(value: string): Promise<void> {
    this.query.set(value);
    if (!value.trim()) {
      this.results.set([]);
      this.showCreateButton.set(false);
      return;
    }
    const resp = await this.api.searchPageNames(value);
    this.results.set(resp.results);
    this.selectedIndex.set(-1);
    const exactMatch = resp.results.some(r => r.pageTitle.toLowerCase() === value.toLowerCase());
    this.showCreateButton.set(!exactMatch && value.trim().length > 0);
  }

  async onFullTextSearch(): Promise<void> {
    const q = this.query().trim();
    if (!q) return;
    const resp = await this.api.searchFullText(q);
    this.results.set(resp.results);
    this.showCreateButton.set(false);
  }

  navigateToResult(result: SearchResult): void {
    this.showDropdown.set(false);
    this.query.set('');
    this.results.set([]);
    this.showCreateButton.set(false);
    this.nav.navigateToPage(result.pageTitle, result.pageId);
  }

  async createPage(): Promise<void> {
    let title = this.query().trim();

    // If empty or whitespace-only, generate timestamp-based title
    if (!title) {
      title = generateUntitledTitle();
    }

    this.showDropdown.set(false);
    this.query.set('');
    this.results.set([]);
    if (classifyJournalFileName(title) !== 'none') {
      this.nav.navigateToPage(title, '');
    } else {
      const error = validateFileName(title);
      if (error) { alert(error); return; }
      const page = await this.api.createPage(title);
      this.recentPages.refresh();
      this.nav.navigateToPage(page.fileName, page.id);
    }
  }

  onKeydown(event: KeyboardEvent): void {
    const maxIndex = this.results().length + (this.showCreateButton() && this.query().trim() ? 0 : -1);

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      this.selectedIndex.update(i => i < maxIndex ? i + 1 : 0);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      this.selectedIndex.update(i => i > 0 ? i - 1 : maxIndex);
    } else if (event.key === 'Enter') {
      const idx = this.selectedIndex();
      if (idx >= 0 && idx < this.results().length) {
        event.preventDefault();
        this.navigateToResult(this.results()[idx]);
      } else if (idx === this.results().length && this.showCreateButton() && this.query().trim()) {
        event.preventDefault();
        this.createPage();
      } else {
        this.onFullTextSearch();
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
