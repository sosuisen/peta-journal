import { Component, inject, input, signal, HostListener, ElementRef } from '@angular/core';
import { Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../core/services/api.service';
import { SearchResult } from '../core/models/search.model';

@Component({
  selector: 'app-wiki-link-autocomplete',
  imports: [FormsModule],
  template: `
    @if (visible()) {
      <div class="wiki-autocomplete" [style.left.px]="posX()" [style.top.px]="posY()">
        @for (result of suggestions(); track result.pageId) {
          <div
            class="suggestion"
            [class.active]="selectedIndex() === $index"
            (mousedown)="selectSuggestion(result)"
          >
            {{ result.pageTitle }}
          </div>
        }
        @if (suggestions().length === 0 && query().length > 0) {
          <div class="suggestion create" (mousedown)="createAndLink()">
            + Create "{{ query() }}"
          </div>
        }
      </div>
    }
  `,
  styles: [`
    .wiki-autocomplete {
      position: fixed;
      background: white;
      border: 1px solid #ddd;
      border-radius: 6px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.15);
      max-height: 200px;
      overflow-y: auto;
      min-width: 200px;
      z-index: 1000;
    }
    .suggestion {
      padding: 6px 12px;
      cursor: pointer;
      font-size: 13px;
    }
    .suggestion:hover, .suggestion.active { background: #f0f0f0; }
    .suggestion.create { color: #007bff; }
  `]
})
export class WikiLinkAutocompleteComponent {
  private readonly api = inject(ApiService);
  private readonly router = inject(Router);

  readonly visible = signal(false);
  readonly query = signal('');
  readonly suggestions = signal<SearchResult[]>([]);
  readonly selectedIndex = signal(0);
  readonly posX = signal(0);
  readonly posY = signal(0);

  readonly onSelect = input<((pageTitle: string) => void) | undefined>(undefined);

  async show(x: number, y: number, initialQuery = ''): Promise<void> {
    this.posX.set(x);
    this.posY.set(y);
    this.query.set(initialQuery);
    this.visible.set(true);
    this.selectedIndex.set(0);
    if (initialQuery) await this.search(initialQuery);
  }

  hide(): void {
    this.visible.set(false);
    this.suggestions.set([]);
    this.query.set('');
  }

  async updateQuery(q: string): Promise<void> {
    this.query.set(q);
    this.selectedIndex.set(0);
    await this.search(q);
  }

  private async search(q: string): Promise<void> {
    if (!q.trim()) {
      this.suggestions.set([]);
      return;
    }
    const resp = await this.api.searchPageNames(q, 10);
    this.suggestions.set(resp.results);
  }

  selectSuggestion(result: SearchResult): void {
    const handler = this.onSelect();
    if (handler) handler(result.pageTitle);
    this.hide();
  }

  async createAndLink(): Promise<void> {
    const title = this.query().trim();
    if (!title) return;
    await this.api.createPage(title);
    const handler = this.onSelect();
    if (handler) handler(title);
    this.hide();
  }

  navigateToPage(pageId: string): void {
    this.router.navigate(['/pages', pageId]);
  }

  moveSelection(delta: number): void {
    const max = this.suggestions().length;
    if (max === 0) return;
    this.selectedIndex.update(i => (i + delta + max) % max);
  }

  confirmSelection(): void {
    const s = this.suggestions();
    const idx = this.selectedIndex();
    if (s.length > 0 && idx < s.length) {
      this.selectSuggestion(s[idx]);
    } else {
      this.createAndLink();
    }
  }
}
