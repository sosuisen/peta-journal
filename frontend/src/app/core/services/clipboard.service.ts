import { Injectable } from '@angular/core';

@Injectable({ providedIn: 'root' })
export class ClipboardService {
  private storedFlatMarkdown: string | null = null;
  private storedTreeMarkdown: string | null = null;

  store(flatMd: string, treeMd: string): void {
    this.storedFlatMarkdown = flatMd;
    this.storedTreeMarkdown = treeMd;
  }

  /**
   * If the current clipboard text/plain matches the stored tree markdown,
   * return the flat markdown equivalent. Otherwise return null.
   */
  consume(currentTextPlain: string): string | null {
    if (this.storedTreeMarkdown && currentTextPlain === this.storedTreeMarkdown) {
      return this.storedFlatMarkdown;
    }
    return null;
  }

  clear(): void {
    this.storedFlatMarkdown = null;
    this.storedTreeMarkdown = null;
  }
}
