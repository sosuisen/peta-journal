import { Component, inject, signal, OnInit, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import { formatJournalDate } from '../../core/utils/date.utils';
import { validateFileName } from '../../core/utils/filename-validation.utils';

@Component({
  selector: 'app-page',
  imports: [OutlineComponent],
  template: `
    @if (page()) {
      <div class="page-header">
        <textarea
          class="page-title-textarea"
          [value]="pageTitle()"
          (keydown.enter)="onTitleEnter($event)"
          (input)="onTitleInput($event)"
          (blur)="saveTitleFromTextarea($event)"
          rows="1"
        ></textarea>
      </div>
      <app-outline [pageId]="page()!.id" />
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .page-header { padding: 16px 0; }
    .page-title-textarea {
      margin: 0; font-size: 1.5em; font-weight: bold; font-family: inherit;
      width: 100%; border: 1px solid transparent; border-radius: 4px;
      padding: 4px; outline: none; box-sizing: border-box;
      resize: none; overflow: hidden; background: transparent;
      line-height: 1.2;
    }
    .page-title-textarea:hover { background: var(--hover-bg, rgba(0,0,0,0.04)); }
    .page-title-textarea:focus {
      background: white;
      border-color: var(--border-color, #ccc);
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
  private originalTitle = '';

  pageTitle(): string {
    const p = this.page();
    if (!p) return '';

    if (p.dirPath.startsWith('journal')) {
      return formatJournalDate(p.fileName);
    }
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

  onTitleEnter(event: Event): void {
    event.preventDefault();
    const textarea = event.target as HTMLTextAreaElement;
    textarea.blur();
  }

  onTitleInput(event: Event): void {
    const textarea = event.target as HTMLTextAreaElement;
    textarea.style.height = 'auto';
    textarea.style.height = textarea.scrollHeight + 'px';
  }

  async saveTitleFromTextarea(event: Event): Promise<void> {
    const textarea = event.target as HTMLTextAreaElement;
    const newTitle = textarea.value.replace(/[\r\n]+/g, ' ').trim();
    if (!newTitle || newTitle === this.originalTitle) {
      textarea.value = this.originalTitle;
      return;
    }
    await this.saveTitle(newTitle);
  }

  private async saveTitle(newTitle: string): Promise<void> {
    const p = this.page();
    if (!p) return;
    const error = validateFileName(newTitle);
    if (error) { alert(error); return; }
    const updated = await this.api.updatePage(p.id, { title: newTitle });
    this.page.set(updated);
    this.originalTitle = newTitle;
    this.title.setTitle(newTitle);
    this.recentPages.refresh();
  }

  private async loadPage(pageId: string): Promise<void> {
    try {
      const result = await this.api.getPage(pageId);
      this.page.set(result.page);
      this.originalTitle = this.pageTitle();
      this.title.setTitle(this.pageTitle());
    } catch (e) {
      console.error('Failed to load page:', pageId, e);
    }
  }
}
