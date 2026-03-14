import { Component, inject, input, signal, computed, effect, Injector, afterNextRender, ElementRef, viewChild } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { OutlineComponent } from '../outline/outline';
import { ApiService } from '../../core/services/api.service';
import { Page } from '../../core/models/page.model';
import { CurrentPageService } from '../../core/services/current-page.service';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import { NavigationService } from '../../core/services/navigation.service';
import { formatJournalDate } from '../../core/utils/date.utils';
import { validateFileName } from '../../core/utils/filename-validation.utils';
import { ExtensionFunctionService } from '../../core/services/extension-function.service';

@Component({
  selector: 'app-page',
  imports: [OutlineComponent],
  template: `
    @if (page()) {
      <div class="page-header" [class.scrolled]="scrolled()">
        @if (editing()) {
          <div class="edit-row" (focusout)="onEditRowFocusOut($event)">
            <input
              #dirPathInput
              class="edit-dir-path"
              [value]="editDirPath()"
              (input)="editDirPath.set(asInputValue($event))"
              (keydown.enter)="save()"
              (keydown.escape)="cancelEdit()"
            />
            <span class="edit-slash">/</span>
            <input
              #fileNameInput
              class="edit-file-name"
              [value]="editFileName()"
              [disabled]="isIndexPage()"
              (input)="editFileName.set(asInputValue($event))"
              (keydown.enter)="save()"
              (keydown.escape)="cancelEdit()"
            />
          </div>
        } @else {
          @if (page()!.dirPath) {
            <div class="dir-path-breadcrumb" [class.index-breadcrumb]="isIndexPage()">
              @for (seg of dirPathSegments(); track seg.path; let last = $last) {
                @if (isIndexPage() && last) {
                  <span class="dir-segment-title" (click)="startEdit($event)">{{ seg.name }}</span><span class="breadcrumb-slash">/</span>
                } @else {
                  <a class="dir-segment" (click)="navigateToSegment(seg.path)">{{ seg.name }}</a><span class="breadcrumb-slash">/</span>
                }
              }
            </div>
          }
          @if (isRootIndex()) {
            <h2 class="page-title root-index-title">(root)/</h2>
            <div class="index-label">index</div>
          } @else if (isIndexPage()) {
            <div class="index-label">
              index
              @if (canDemoteIndex()) {
                <button class="demote-btn" (click)="demoteIndex()">ページ名を {{ demoteName() }} に変更する</button>
              }
            </div>
          } @else {
            <h2 class="page-title" (click)="startEdit($event)">{{ pageTitle() }}</h2>
          }
        }
      </div>
      <app-outline [pageId]="page()!.id" [focusItemId]="focusItemId()" (scrolledFromTop)="scrolled.set($event)" />
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .page-header { padding: 16px 0; }
    .page-header.scrolled { border-bottom: 1px dashed #909090; }

    /* View mode — match edit-mode padding+border so no layout jump */
    .dir-path-breadcrumb {
      display: flex;
      align-items: baseline;
      padding: 4px 6px;
      border: 1px solid transparent;
      flex-wrap: wrap;
    }
    .dir-segment {
      color: var(--text-secondary, #666);
      cursor: pointer;
      text-decoration: none;
      font-size: 1.0em;
    }
    .dir-segment:hover { text-decoration: underline; }
    .breadcrumb-slash {
      color: var(--text-secondary, #666);
      font-size: 1.0em;
      margin: 0 1px;
    }
    .page-title {
      margin: 0;
      font-size: 2em;
      font-weight: bold;
      padding: 4px 6px;
      border: 1px solid transparent;
      cursor: pointer;
      border-radius: 4px;
      line-height: 1.2;
      width: 100%;
      box-sizing: border-box;
    }
    .page-title:hover { background: var(--hover-bg, rgba(0,0,0,0.04)); }
    .dir-segment-title {
      font-size: 2em;
      font-weight: bold;
      color: #000;
      cursor: pointer;
    }
    .dir-segment-title:hover { text-decoration: underline; }
    .index-breadcrumb .dir-segment,
    .index-breadcrumb .breadcrumb-slash {
      font-size: 2em;
      font-weight: bold;
      color: #000;
    }
    .root-index-title {
      color: #999;
      cursor: default;
    }
    .root-index-title:hover {
      background: transparent;
    }
    .index-label {
      padding: 4px 6px;
      font-size: 1em;
      color: #999;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .demote-btn {
      font-size: 12px;
      color: #666;
      background: none;
      border: 1px solid #ccc;
      border-radius: 4px;
      padding: 2px 8px;
      cursor: pointer;
    }
    .demote-btn:hover {
      background: #f0f0f0;
      border-color: #999;
    }

    /* Edit mode */
    .edit-row {
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
    }
    .edit-dir-path, .edit-file-name {
      font-family: inherit;
      border: 1px solid #e8ddc3;
      border-radius: 4px;
      padding: 4px 6px;
      outline: none;
      background: #fffde7;
    }
    .edit-dir-path {
      font-size: 1.0em;
      color: var(--text-secondary, #666);
      width: auto;
      min-width: 60px;
      field-sizing: content;
    }
    .edit-file-name {
      font-size: 2em;
      font-weight: bold;
      width: 100%;
      box-sizing: border-box;
    }
    .edit-file-name:disabled {
      background: #f0f0f0;
      color: #999;
      border-color: #ddd;
      cursor: default;
    }
    .edit-slash {
      font-size: 1.0em;
      color: var(--text-secondary, #666);
      margin: 0 2px;
    }
  `]
})
export class PageComponent {
  readonly pageName = input.required<string>();
  readonly focusItemId = input<string | null | undefined>('');
  private readonly api = inject(ApiService);
  private readonly titleService = inject(Title);
  private readonly currentPageService = inject(CurrentPageService);
  private readonly recentPages = inject(RecentPagesService);
  private readonly navigation = inject(NavigationService);
  private readonly extensionFunctions = inject(ExtensionFunctionService);
  private readonly injector = inject(Injector);

  readonly fileNameInput = viewChild<ElementRef<HTMLInputElement>>('fileNameInput');
  readonly dirPathInput = viewChild<ElementRef<HTMLInputElement>>('dirPathInput');

  readonly page = signal<Page | null>(null);
  readonly scrolled = signal(false);
  readonly editing = signal(false);
  readonly editDirPath = signal('');
  readonly editFileName = signal('');
  readonly isIndexPage = computed(() => this.page()?.name === 'index');
  readonly isRootIndex = computed(() => this.page()?.name === 'index' && !this.page()?.dirPath);
  readonly canDemoteIndex = signal(false);
  readonly demoteName = computed(() => {
    const p = this.page();
    if (!p || !p.dirPath) return '';
    const trimmed = p.dirPath.replace(/\/$/, '');
    const lastSlash = trimmed.lastIndexOf('/');
    return lastSlash < 0 ? trimmed : trimmed.substring(lastSlash + 1);
  });

  readonly dirPathSegments = computed(() => {
    const p = this.page();
    if (!p || !p.dirPath) return [];
    const parts = p.dirPath.replace(/\/$/, '').split('/');
    const segments: { name: string; path: string }[] = [];
    for (let i = 0; i < parts.length; i++) {
      segments.push({
        name: parts[i],
        path: parts.slice(0, i + 1).join('/'),
      });
    }
    return segments;
  });

  /**
   * Returns the display title for the current page.
   * Journals are formatted as human-readable dates; documents use their file name.
   * @returns The formatted page title string.
   */
  pageTitle(): string {
    const p = this.page();
    if (!p) return '';
    if (p.type === 'journal') {
      return formatJournalDate(p.name);
    }
    if (p.name === 'index' && p.dirPath) {
      const segments = p.dirPath.replace(/\/$/, '').split('/');
      return segments[segments.length - 1] + '/';
    }
    return p.name;
  }

  /** Full dir_path + name for browser <title>. Index pages show "dir_path/" only. */
  fullPageTitle(): string {
    const p = this.page();
    if (!p) return '';
    if (p.type === 'journal') {
      return formatJournalDate(p.name);
    }
    if (p.name === 'index') {
      return p.dirPath ? p.dirPath : '(root)/';
    }
    return p.dirPath + p.name;
  }

  constructor() {
    effect(() => {
      const name = this.pageName();
      if (name) this.loadPage(name);
    });
  }

  /**
   * Extracts the string value from an input element's event.
   * @param event - The DOM input event.
   * @returns The current value of the input element.
   */
  asInputValue(event: Event): string {
    return (event.target as HTMLInputElement).value;
  }

  /**
   * Navigates to a page identified by a directory path breadcrumb segment.
   * @param path - The directory path to navigate to.
   */
  navigateToSegment(path: string): void {
    this.navigation.navigateToPage(path + '/index');
  }

  /**
   * Enters edit mode for the page title, placing the caret at the click position.
   * @param event - The mouse click event on the page title.
   */
  startEdit(event: MouseEvent): void {
    const p = this.page();
    if (!p) return;
    this.editDirPath.set(p.dirPath.replace(/\/$/, ''));
    this.editFileName.set(p.name);

    // Determine caret position from click coordinates
    let caretPos = p.name.length;
    const range = document.caretRangeFromPoint(event.clientX, event.clientY);
    if (range) {
      caretPos = range.startOffset;
    }

    this.editing.set(true);
    afterNextRender(() => {
      if (p.name === 'index') {
        const input = this.dirPathInput()?.nativeElement;
        if (input) {
          input.focus();
          input.setSelectionRange(input.value.length, input.value.length);
        }
      } else {
        const input = this.fileNameInput()?.nativeElement;
        if (input) {
          input.focus();
          input.setSelectionRange(caretPos, caretPos);
        }
      }
    }, { injector: this.injector });
  }

  /**
   * Saves the edited page title and/or directory path to the server.
   * Validates the file name and exits edit mode on success.
   */
  async save(): Promise<void> {
    const p = this.page();
    if (!p) return;

    const newFileName = this.editFileName().trim();
    if (!newFileName) {
      alert('Page name cannot be empty');
      return;
    }
    const error = validateFileName(newFileName);
    if (error) {
      alert(error);
      return;
    }

    let newDirPath = this.editDirPath().trim();
    if (newDirPath && !newDirPath.endsWith('/')) {
      newDirPath += '/';
    }

    const titleChanged = newFileName !== p.name;
    const dirChanged = newDirPath !== p.dirPath;

    if (!titleChanged && !dirChanged) {
      this.editing.set(false);
      return;
    }

    const updates: { title?: string; dirPath?: string } = {};
    if (titleChanged) updates.title = newFileName;
    if (dirChanged) updates.dirPath = newDirPath;

    const updated = await this.api.updatePage(p.id, updates);
    this.page.set(updated);
    this.titleService.setTitle(this.fullPageTitle());
    this.recentPages.refresh();
    this.editing.set(false);

    // Navigate to the updated URL
    const canonical = updated.dirPath
      ? updated.dirPath.replace(/\/$/, '') + '/' + updated.name
      : updated.name;
    this.navigation.navigateToPage(canonical, false, false, true);
  }

  /** Cancels edit mode and discards unsaved title changes. */
  cancelEdit(): void {
    this.editing.set(false);
  }

  /**
   * Auto-saves when focus leaves the edit row, unless focus moved to another element within it.
   * @param event - The focus-out event from the edit row container.
   */
  onEditRowFocusOut(event: FocusEvent): void {
    const editRow = (event.currentTarget as HTMLElement);
    const newFocus = event.relatedTarget as Node | null;
    if (!newFocus || !editRow.contains(newFocus)) {
      this.save();
    }
  }

  private async loadPage(pageName: string): Promise<void> {
    try {
      const result = await this.api.getOrCreateDoc(pageName);
      const p = result.page;

      // Compute canonical URL path from loaded page
      const canonical = p.dirPath
        ? p.dirPath.replace(/\/$/, '') + '/' + p.name
        : p.name;

      // If URL doesn't match canonical path, redirect
      // e.g. server resolved "Quarkus" → dirPath="Quarkus/" name="index" → redirect to "Quarkus/index"
      if (canonical !== pageName) {
        this.navigation.navigateToPage(canonical, false, false, true);
        return;
      }

      this.page.set(p);
      this.currentPageService.setCurrentPage(p);
      if (result.created) {
        this.recentPages.refresh();
        this.extensionFunctions.emitPageCreate({
          pageId: p.id,
          fileName: p.name,
          dirName: p.dirPath,
          pageType: p.type
        });
      }
      this.titleService.setTitle(this.fullPageTitle());

      // Check if non-root index page can be demoted
      if (p.name === 'index' && p.dirPath) {
        const children = await this.api.getChildPages(p.id);
        this.canDemoteIndex.set(children.length === 0);
      } else {
        this.canDemoteIndex.set(false);
      }
    } catch (e) {
      console.error('Failed to load page:', pageName, e);
    }
  }

  async demoteIndex(): Promise<void> {
    const p = this.page();
    if (!p || !p.dirPath) return;
    const name = this.demoteName();
    if (!name) return;

    // "a/b/c/" → parentDir = "a/b/"
    const trimmed = p.dirPath.replace(/\/$/, '');
    const lastSlash = trimmed.lastIndexOf('/');
    const parentDir = lastSlash < 0 ? '' : trimmed.substring(0, lastSlash + 1);

    try {
      await this.api.updatePage(p.id, { title: name, dirPath: parentDir });
    } catch (e: any) {
      alert(e.message || 'Failed to rename page');
      return;
    }
    this.recentPages.refresh();
    this.navigation.navigateToPage(parentDir + name);
  }
}
