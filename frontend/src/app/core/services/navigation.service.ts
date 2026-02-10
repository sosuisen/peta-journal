import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import { classifyJournalFileName } from '../utils/journal-filename.utils';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly router = inject(Router);

  /** Navigate to a page by fileName. Journals go to /journals/, docs go to /docs/. */
  navigateToPage(fileName: string, newTab = false): void {
    const prefix = classifyJournalFileName(fileName) !== 'none' ? '/journals' : '/docs';
    if (newTab) {
      window.open(`${prefix}/${encodeURIComponent(fileName)}`, '_blank');
    } else {
      this.router.navigate([prefix, fileName]);
    }
  }
}
