import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import { classifyJournalFileName } from '../utils/journal-filename.utils';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly router = inject(Router);

  /** Navigate to a page by title (optionally with dirPath prefix like "cat/mypage"). */
  navigateToPage(pageTitle: string, newTab = false): void {
    const isJournal = classifyJournalFileName(pageTitle) !== 'none';
    const prefix = isJournal ? '/journals' : '/docs';
    if (newTab) {
      window.open(`${prefix}/${pageTitle}`, '_blank');
    } else {
      this.router.navigateByUrl(`${prefix}/${pageTitle}`);
    }
  }
}
