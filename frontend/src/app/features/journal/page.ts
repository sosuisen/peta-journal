import { Component, inject, signal, OnInit, OnDestroy, ElementRef, viewChild } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { RecentPagesService } from '../../core/services/recent-pages.service';

@Component({
  selector: 'app-page',
  imports: [OutlineComponent],
  template: `
    @if (page()) {
      <div class="page-header">
        @if (editingTitle()) {
          <input
            #titleInput
            class="page-title-input"
            [value]="pageTitle()"
            (keydown.enter)="saveTitleFromInput($event)"
            (keydown.escape)="cancelTitleEdit()"
            (blur)="saveTitleFromInput($event)"
          />
        } @else {
          <h1 class="page-title" (click)="startTitleEdit()">{{ pageTitle() }}</h1>
        }
      </div>
      <app-outline [pageId]="page()!.id" />
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .page-header { padding: 16px 0; }
    .page-title { margin: 0; font-size: 1.5em; cursor: pointer; }
    .page-title:hover { background: var(--hover-bg, rgba(0,0,0,0.04)); border-radius: 4px; }
    .page-title-input {
      margin: 0; font-size: 1.5em; font-weight: bold; font-family: inherit;
      width: 100%; border: 1px solid var(--border-color, #ccc); border-radius: 4px;
      padding: 2px 4px; outline: none; box-sizing: border-box;
    }
  `]
})
export class PageComponent implements OnInit, OnDestroy {
  private readonly route = inject(ActivatedRoute);
  private readonly api = inject(ApiService);
  private readonly title = inject(Title);
  private readonly recentPages = inject(RecentPagesService);
  private routeSub?: Subscription;

  readonly page = signal<Page | null>(null);
  readonly editingTitle = signal(false);
  readonly titleInput = viewChild<ElementRef<HTMLInputElement>>('titleInput');

  pageTitle(): string {
    const p = this.page();
    if (!p) return '';

    // For journal pages (dirPath starts with 'journal'), format as locale date
    if (p.dirPath.startsWith('journal')) {
      try {
        const date = new Date(p.fileName + 'T00:00:00');
        const dateStr = date.toLocaleDateString('ja-JP', {
          year: 'numeric',
          month: 'long',
          day: 'numeric'
        });
        const weekdayStr = date.toLocaleDateString('ja-JP', {
          weekday: 'long'
        });
        return `${dateStr} ${weekdayStr}`;
      } catch {
        return p.fileName;
      }
    }
    // For regular pages, use fileName as-is
    return p.fileName;
  }

  ngOnInit(): void {
    this.routeSub = this.route.paramMap.subscribe(params => {
      const id = params.get('id');
      if (id) this.loadPage(id);
    });
  }

  ngOnDestroy(): void {
    this.routeSub?.unsubscribe();
  }

  startTitleEdit(): void {
    this.editingTitle.set(true);
    queueMicrotask(() => {
      const input = this.titleInput()?.nativeElement;
      if (input) {
        input.focus();
        input.select();
      }
    });
  }

  async saveTitleFromInput(event: Event): Promise<void> {
    if (!this.editingTitle()) return;
    const input = event.target as HTMLInputElement;
    const newTitle = input.value.trim();
    if (!newTitle || newTitle === this.pageTitle()) {
      this.editingTitle.set(false);
      return;
    }
    await this.saveTitle(newTitle);
  }

  cancelTitleEdit(): void {
    this.editingTitle.set(false);
  }

  private async saveTitle(newTitle: string): Promise<void> {
    const p = this.page();
    if (!p) return;
    this.editingTitle.set(false);
    try {
      const updated = await this.api.updatePage(p.id, { title: newTitle });
      this.page.set(updated);
      this.title.setTitle(newTitle);
      this.recentPages.refresh();
    } catch (e) {
      console.error('Failed to update page title:', e);
    }
  }

  private async loadPage(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPage(pageId);
      this.page.set(result.page);
      this.title.setTitle(this.pageTitle());
    } catch (e) {
      console.error('Failed to load page:', pageId, e);
    }
  }
}
