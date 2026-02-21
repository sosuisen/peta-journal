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
      <div class="page-header">
        @if (editing()) {
          <div class="edit-row" (focusout)="onEditRowFocusOut($event)">
            <input
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
              (input)="editFileName.set(asInputValue($event))"
              (keydown.enter)="save()"
              (keydown.escape)="cancelEdit()"
            />
          </div>
        } @else {
          @if (page()!.dirPath) {
            <div class="dir-path-breadcrumb">
              @for (seg of dirPathSegments(); track seg.path) {
                <a class="dir-segment" (click)="navigateToSegment(seg.path)">{{ seg.name }}</a><span class="breadcrumb-slash">/</span>
              }
            </div>
          }
          <h2 class="page-title" (click)="startEdit($event)">{{ pageTitle() }}</h2>
        }
      </div>
      <app-outline [pageId]="page()!.id" />
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .page-header { padding: 16px 0; }

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
    .edit-slash {
      font-size: 1.0em;
      color: var(--text-secondary, #666);
      margin: 0 2px;
    }
  `]
})
export class PageComponent {
  readonly pageName = input.required<string>();
  private readonly api = inject(ApiService);
  private readonly titleService = inject(Title);
  private readonly currentPageService = inject(CurrentPageService);
  private readonly recentPages = inject(RecentPagesService);
  private readonly navigation = inject(NavigationService);
  private readonly extensionFunctions = inject(ExtensionFunctionService);
  private readonly injector = inject(Injector);

  readonly fileNameInput = viewChild<ElementRef<HTMLInputElement>>('fileNameInput');

  readonly page = signal<Page | null>(null);
  readonly editing = signal(false);
  readonly editDirPath = signal('');
  readonly editFileName = signal('');

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
      return formatJournalDate(p.fileName);
    }
    return p.fileName;
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
    this.editFileName.set(p.fileName);

    // Determine caret position from click coordinates
    let caretPos = p.fileName.length;
    const range = document.caretRangeFromPoint(event.clientX, event.clientY);
    if (range) {
      caretPos = range.startOffset;
    }

    this.editing.set(true);
    afterNextRender(() => {
      const input = this.fileNameInput()?.nativeElement;
      if (input) {
        input.focus();
        input.setSelectionRange(caretPos, caretPos);
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

    const titleChanged = newFileName !== p.fileName;
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
    this.titleService.setTitle(this.pageTitle());
    this.recentPages.refresh();
    this.editing.set(false);
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
        ? p.dirPath.replace(/\/$/, '') + '/' + p.fileName
        : p.fileName;

      // If URL doesn't match canonical path, redirect
      // e.g. server resolved "Quarkus" → dirPath="Quarkus/" fileName="index" → redirect to "Quarkus/index"
      if (canonical !== pageName) {
        this.navigation.navigateToPage(canonical, false, true);
        return;
      }

      this.page.set(p);
      this.currentPageService.setCurrentPage(p);
      if (result.created) {
        this.recentPages.refresh();
        this.extensionFunctions.emitPageCreate({
          pageId: p.id,
          fileName: p.fileName,
          dirName: p.dirPath
        });
      }
      this.titleService.setTitle(this.pageTitle());
    } catch (e) {
      console.error('Failed to load page:', pageName, e);
    }
  }
}
