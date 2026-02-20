import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly router = inject(Router);

  /** Navigate to a page by title (optionally with dirPath prefix like "cat/mypage"). */
  navigateToPage(pageTitle: string, newTab = false, replaceUrl = false): void {
    const url = '/pages/' + pageTitle;
    if (newTab) {
      window.open(url, '_blank');
    } else {
      this.router.navigateByUrl(url, { replaceUrl });
    }
  }
}
