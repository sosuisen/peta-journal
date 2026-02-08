import { Component, input, output, inject, computed, viewChild, ElementRef, effect, afterNextRender, Injector } from '@angular/core';
import { Router } from '@angular/router';
import { DomSanitizer } from '@angular/platform-browser';
import { ApiService } from '../../core/services/api.service';
import { RecentPagesService } from '../../core/services/recent-pages.service';
import hljs from 'highlight.js';

@Component({
  selector: 'app-block-view',
  template: `
    @if (status() !== 'normal') {
      <input
        #checkbox
        type="checkbox"
        class="task-checkbox"
        [checked]="status() === 'done'"
        (click)="onCheckboxClick($event)"
      />
    }
    <div
      #content
      class="block-content"
      [innerHTML]="trustedHtml()"
      (click)="onClick($event)"
    ></div>
  `,
  styles: [`
    :host {
      display: flex;
      align-items: flex-start;
      flex: 1;
      min-width: 0;
    }
    .block-content {
      cursor: text;
      line-height: 1.5;
      padding: 2px 0;
      min-height: 1.5em;
      word-wrap: break-word;
      flex: 1;
      min-width: 0;
    }
    .block-content:empty::after {
      content: ' ';
      white-space: pre;
    }
    :host ::ng-deep .wiki-link {
      color: #007bff;
      cursor: pointer;
      text-decoration: underline;
    }
    :host ::ng-deep .wiki-link:hover {
      color: #0056b3;
    }
    :host ::ng-deep .wiki-link-new {
      color: #d9534f;
    }
    :host ::ng-deep .wiki-link-new:hover {
      color: #c9302c;
    }
    :host ::ng-deep input[type="checkbox"] {
      cursor: pointer;
      margin-right: 4px;
    }
    :host ::ng-deep code {
      background: #f5f5f5;
      padding: 2px 6px;
      border-radius: 3px;
      font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
      font-size: 0.9em;
      color: #d63384;
    }
    :host ::ng-deep pre {
      background: #fffbf0;
      padding: 16px;
      border-radius: 6px;
      border: 1px solid #e8ddc3;
      overflow-x: auto;
      margin: 8px 0;
      position: relative;
    }
    :host ::ng-deep pre code {
      background: none;
      padding: 0;
      color: inherit;
      font-size: 0.875em;
      border-radius: 0;
      display: block;
    }
    :host ::ng-deep pre code.hljs {
      background: none;
    }
    :host ::ng-deep .code-lang-label {
      position: absolute;
      top: 8px;
      right: 12px;
      background: transparent;
      border: none;
      border-radius: 3px;
      padding: 2px 6px;
      font-size: 10px;
      font-weight: 400;
      color: #000;
      text-transform: lowercase;
      letter-spacing: 0;
      opacity: 0.5;
      pointer-events: none;
      user-select: none;
    }
    :host ::ng-deep blockquote {
      margin: 8px 0;
      padding: 8px 16px;
      border-left: 4px solid #d0d7de;
      background: #f6f8fa;
      color: #000;
    }
    :host ::ng-deep blockquote p {
      margin: 0;
    }
    .task-checkbox {
      cursor: pointer;
      margin-right: 6px;
      margin-top: 4px;
      flex-shrink: 0;
    }
  `]
})
export class BlockViewComponent {
  private readonly router = inject(Router);
  private readonly api = inject(ApiService);
  private readonly sanitizer = inject(DomSanitizer);
  private readonly recentPages = inject(RecentPagesService);
  private readonly injector = inject(Injector);

  readonly html = input.required<string>();
  readonly status = input.required<string>();
  readonly clicked = output<number>();
  readonly taskToggled = output<void>();
  readonly trustedHtml = computed(() => this.sanitizer.bypassSecurityTrustHtml(this.html()));

  private readonly checkboxRef = viewChild<ElementRef<HTMLInputElement>>('checkbox');
  private readonly contentRef = viewChild<ElementRef<HTMLDivElement>>('content');

  constructor() {
    effect(() => {
      const checkbox = this.checkboxRef()?.nativeElement;
      if (checkbox) {
        checkbox.indeterminate = this.status() === 'running';
      }
    });

    // Apply syntax highlighting when HTML changes
    effect(() => {
      const html = this.html();
      if (html) {
        afterNextRender(() => {
          const content = this.contentRef()?.nativeElement;
          if (content) {
            content.querySelectorAll('pre code').forEach((block) => {
              const codeEl = block as HTMLElement;
              const preEl = codeEl.parentElement as HTMLPreElement;

              // Apply syntax highlighting
              hljs.highlightElement(codeEl);

              // Add language label if not already added
              if (preEl && !preEl.querySelector('.code-lang-label')) {
                const classes = codeEl.className.split(' ');
                const langClass = classes.find(c => c.startsWith('language-'));
                if (langClass) {
                  const lang = langClass.replace('language-', '');
                  const label = document.createElement('div');
                  label.className = 'code-lang-label';
                  label.textContent = lang;
                  preEl.style.position = 'relative';
                  preEl.appendChild(label);
                }
              }
            });
          }
        }, { injector: this.injector });
      }
    });
  }

  onCheckboxClick(event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.taskToggled.emit();
  }

  async onClick(event: MouseEvent): Promise<void> {
    const target = event.target as HTMLElement;

    const link = target.closest('.wiki-link') as HTMLElement | null;
    if (!link) {
      this.clicked.emit(this.getTextOffsetAtPoint(event.clientX, event.clientY));
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    const pageId = link.dataset['pageId'];
    if (pageId) {
      this.router.navigate(['/pages', pageId]);
      return;
    }

    const pageTitle = link.dataset['pageTitle'];
    if (pageTitle) {
      const page = await this.api.createPage(pageTitle);
      this.recentPages.refresh();
      this.router.navigate(['/pages', page.id]);
    }
  }

  private getTextOffsetAtPoint(x: number, y: number): number {
    let targetNode: Node;
    let offsetInNode: number;

    // Firefox uses caretPositionFromPoint, Chrome/Safari use caretRangeFromPoint
    const doc = document as any;
    if (doc.caretPositionFromPoint) {
      const pos = doc.caretPositionFromPoint(x, y);
      if (!pos) return 0;
      targetNode = pos.offsetNode;
      offsetInNode = pos.offset;
    } else if (doc.caretRangeFromPoint) {
      const range = doc.caretRangeFromPoint(x, y);
      if (!range) return 0;
      targetNode = range.startContainer;
      offsetInNode = range.startOffset;
    } else {
      return 0;
    }

    // If the target is an element (not a text node), return 0
    if (targetNode.nodeType !== Node.TEXT_NODE) return 0;

    const container = targetNode.parentElement?.closest('.block-content');
    if (!container) return offsetInNode;

    const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
    let cumulative = 0;
    let node: Node | null;
    while ((node = walker.nextNode())) {
      if (node === targetNode) return cumulative + offsetInNode;
      cumulative += node.textContent?.length ?? 0;
    }
    return cumulative + offsetInNode;
  }
}
