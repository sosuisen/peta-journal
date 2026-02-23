import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';

@Injectable({ providedIn: 'root' })
export class NavigationService {
  private readonly router = inject(Router);

  /** Navigate to a page by title (optionally with dirPath prefix like "cat/mypage"). */
  navigateToPage(pageTitle: string, newTab = false, replaceUrl = false, fragment?: string): void {
    const url = '/pages/' + pageTitle;
    if (newTab) {
      window.open(url + (fragment ? '#' + fragment : ''), '_blank');
    } else {
      this.router.navigate([url], { replaceUrl, fragment });
    }
  }
}
