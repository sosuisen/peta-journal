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
import { ExtensionFunctionService } from '../../core/services/extension-function.service';

@Component({
  selector: 'app-page',
  imports: [OutlineComponent],
  template: `
    @if (page()) {
      <div class="page-header">
        @if (page()!.dirPath) {
          <div class="dir-path-row">
            <textarea
              class="dir-path-textarea"
              [value]="dirPathWithoutSlash()"
              (keydown.enter)="onDirPathEnter($event)"
              (blur)="saveDirPath($event)"
              rows="1"
            ></textarea>
            <span class="dir-path-slash">/</span>
          </div>
        }
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
    .dir-path-row {
      display: flex;
      align-items: baseline;
      padding: 0 2px;
    }
    .dir-path-textarea {
      font-size: 1.0em;
      color: var(--text-secondary, #666);
      font-family: inherit;
      border: 1px solid transparent;
      border-radius: 4px;
      padding: 2px 2px;
      outline: none;
      resize: none;
      overflow: hidden;
      background: transparent;
      line-height: 1.2;
      width: auto;
      max-width: 80%;
      field-sizing: content;
    }
    .dir-path-textarea:hover { background: var(--hover-bg, rgba(0,0,0,0.04)); }
    .dir-path-textarea:focus {
      background: white;
      border-color: var(--border-color, #ccc);
    }
    .dir-path-slash {
      font-size: 1.0em;
      color: var(--text-secondary, #666);
    }
    .page-title-textarea {
      margin: 0; font-size: 2em; font-weight: bold; font-family: inherit;
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
  private readonly extensionFunctions = inject(ExtensionFunctionService);
  private routeSub?: Subscription;

  readonly page = signal<Page | null>(null);
  private originalTitle = '';

  pageTitle(): string {
    const p = this.page();
    if (!p) return '';

    if (p.type === 'journal') {
      return formatJournalDate(p.fileName);
    }
    return p.fileName;
  }

  ngOnInit(): void {
    this.routeSub = this.route.url.subscribe(segments => {
      // Strip leading "docs" segment, join rest: ["docs", "cat", "mypage"] → "cat/mypage"
      const parts = segments.map(s => s.path).filter(s => s !== 'docs');
      const fullPath = parts.join('/');
      if (fullPath) this.loadPage(fullPath);
    });
  }

  ngOnDestroy(): void {
    this.routeSub?.unsubscribe();
  }

  dirPathWithoutSlash(): string {
    const p = this.page();
    if (!p) return '';
    return p.dirPath.replace(/\/$/, '');
  }

  onDirPathEnter(event: Event): void {
    event.preventDefault();
    (event.target as HTMLTextAreaElement).blur();
  }

  async saveDirPath(event: Event): Promise<void> {
    const p = this.page();
    if (!p) return;
    const textarea = event.target as HTMLTextAreaElement;
    let newDirPath = textarea.value.replace(/[\r\n]+/g, ' ').trim();
    // Normalize: ensure trailing slash if non-empty
    if (newDirPath && !newDirPath.endsWith('/')) {
      newDirPath += '/';
    }
    if (newDirPath === p.dirPath) {
      textarea.value = this.dirPathWithoutSlash();
      return;
    }
    const updated = await this.api.updatePage(p.id, { dirPath: newDirPath });
    this.page.set(updated);
    this.recentPages.refresh();
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

    const slashIdx = newTitle.lastIndexOf('/');
    if (slashIdx >= 0) {
      // "lesson/abc" → dirPath += "lesson/", title = "abc"
      const extraDir = newTitle.substring(0, slashIdx + 1);
      const fileName = newTitle.substring(slashIdx + 1);
      if (!fileName) { alert('Page name cannot be empty'); return; }
      const error = validateFileName(fileName);
      if (error) { alert(error); return; }
      const newDirPath = (p.dirPath || '') + extraDir;
      const updated = await this.api.updatePage(p.id, { title: fileName, dirPath: newDirPath });
      this.page.set(updated);
      this.originalTitle = fileName;
      this.title.setTitle(fileName);
      this.recentPages.refresh();
      return;
    }

    const error = validateFileName(newTitle);
    if (error) { alert(error); return; }
    const updated = await this.api.updatePage(p.id, { title: newTitle });
    this.page.set(updated);
    this.originalTitle = newTitle;
    this.title.setTitle(newTitle);
    this.recentPages.refresh();
  }

  private async loadPage(fileName: string): Promise<void> {
    try {
      const result = await this.api.getDocByName(fileName);
      this.page.set(result.page);
      if (result.created) {
        this.extensionFunctions.emitPageCreate({
          pageId: result.page.id,
          fileName: result.page.fileName,
          dirName: result.page.dirPath
        });
      }
      this.originalTitle = this.pageTitle();
      this.title.setTitle(this.pageTitle());
    } catch (e) {
      console.error('Failed to load page:', fileName, e);
    }
  }
}
