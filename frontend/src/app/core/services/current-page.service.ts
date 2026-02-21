import { Injectable, signal } from '@angular/core';
import { Page } from '../models/page.model';

@Injectable({ providedIn: 'root' })
export class CurrentPageService {
  private readonly _currentPage = signal<Page | null>(null);
  readonly currentPage = this._currentPage.asReadonly();

  setCurrentPage(page: Page | null): void {
    this._currentPage.set(page);
  }
}
