import { Component, input, output, inject, computed } from '@angular/core';
import { Router } from '@angular/router';
import { DomSanitizer } from '@angular/platform-browser';
import { ApiService } from '../../core/services/api.service';
import { RecentPagesService } from '../../core/services/recent-pages.service';

@Component({
  selector: 'app-block-view',
  template: `
    <div
      class="block-content"
      [innerHTML]="trustedHtml()"
      (click)="onClick($event)"
    ></div>
  `,
  styles: [`
    :host { display: block; flex: 1; min-width: 0; }
    .block-content {
      cursor: text;
      line-height: 1.5;
      padding: 2px 0;
      min-height: 1.5em;
      word-wrap: break-word;
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
  `]
})
export class BlockViewComponent {
  private readonly router = inject(Router);
  private readonly api = inject(ApiService);
  private readonly sanitizer = inject(DomSanitizer);
  private readonly recentPages = inject(RecentPagesService);

  readonly html = input.required<string>();
  readonly clicked = output<number>();
  readonly trustedHtml = computed(() => this.sanitizer.bypassSecurityTrustHtml(this.html()));

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
