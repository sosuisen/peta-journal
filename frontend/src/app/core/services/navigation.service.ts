import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import { classifyJournalFileName } from '../utils/journal-filename.utils';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly router = inject(Router);

  /** Navigate to a page by fileName (used as the source of truth for routing). */
  navigateToPage(fileName: string, pageId: string, newTab = false): void {
    if (classifyJournalFileName(fileName) !== 'none') {
      if (newTab) {
        window.open(`/journals/${fileName}`, '_blank');
      } else {
        this.router.navigate(['/journals', fileName]);
      }
    } else {
      if (newTab) {
        window.open(`/docs/${pageId}`, '_blank');
      } else {
        this.router.navigate(['/docs', pageId]);
      }
    }
  }
}
