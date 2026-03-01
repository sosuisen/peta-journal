import { Page } from './page.model';

export interface BookmarkGroup {
  id: string;
  name: string;         // empty = default group
  sortOrder: string;    // fractional index
  createdAt: number;
}

export interface Bookmark {
  id: string;
  groupId: string;
  targetType: string;   // "page" or "item"
  targetId: string;
  sortOrder: string;    // fractional index within group
  createdAt: number;
  targetPage: Page | null;
}
