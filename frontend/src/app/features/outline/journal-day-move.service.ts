import { Injectable } from '@angular/core';
import { ApiService, ItemPosition } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { Item } from '../../core/models/item.model';
import { shiftIsoDate } from '../../core/utils/date.utils';
import { classifyJournalFileName } from '../../core/utils/journal-filename.utils';
import { findCrossPagePlacement } from './move-utils';

/** What a move to another day's page did; the journal reloads from it. */
export interface CrossDayMove {
  /** The date of the page the item went to. */
  targetDate: string;
  /** True when the journal should navigate to that page. */
  navigate: boolean;
  itemId: string;
}

/** True for an item of a daily journal page (named YYYY-MM-DD). */
export function isDailyJournalItem(item: Item): boolean {
  return item.type === 'journal' && classifyJournalFileName(item.name) === 'daily';
}

/**
 * Moves an item at the edge of a daily journal page to the neighbouring day: moving the
 * topmost item up takes it to the next day, moving the bottommost item down takes it to
 * the previous day. The journal shows the days newest first, so this keeps the order.
 */
@Injectable({ providedIn: 'root' })
export class JournalDayMoveService {
  constructor(
    private readonly state: OutlineStateService,
    private readonly api: ApiService,
  ) {}

  /**
   * Moves a daily journal item to the adjacent day's page if it is at the edge of its own
   * page. A level 1 item always goes; a level 2 item goes only when the adjacent page is
   * loaded and has a place for it.
   * @param item - The item being moved
   * @param direction - The move the user asked for
   * @param shownPageId - The page the outline shows; a move up from it navigates
   * @returns What the move did, or null when the item stays on its page
   */
  async moveAcrossDays(item: Item, direction: 'up' | 'down', shownPageId: string): Promise<CrossDayMove | null> {
    const pageItems = this.state.items().filter(b => b.pageId === item.pageId);

    if (direction === 'up') {
      // The topmost root item moves to the end of the next day's page
      if (item.level === 1 && pageItems.find(b => b.level === 1)?.id === item.id) {
        return this.moveToAdjacentDay(item, 'next', shownPageId);
      }
      // A level 2 item at the second position moves to the next day's page
      if (item.level === 2 && pageItems.length >= 2 && pageItems[1].id === item.id) {
        return this.moveLevel2ToAdjacentDay(item, 'next');
      }
      return null;
    }

    // The bottommost item moves to the previous day's page
    if (pageItems.length === 0 || pageItems[pageItems.length - 1].id !== item.id) return null;
    if (item.level === 1) return this.moveToAdjacentDay(item, 'prev', shownPageId);
    if (item.level === 2) return this.moveLevel2ToAdjacentDay(item, 'prev');
    return null;
  }

  /**
   * Moves a journal item (and its children) to an adjacent day's page.
   * For 'next': moves to the next day's page at 'last' position.
   * For 'prev': moves to the previous day's page at 'first' position.
   * If the target page has only one item with empty body, that item is deleted after the move.
   */
  private async moveToAdjacentDay(item: Item, direction: 'next' | 'prev', shownPageId: string): Promise<CrossDayMove> {
    const targetDate = this.adjacentDate(item, direction);

    // Get or create the target day's journal
    const result = await this.api.getOrCreateJournal(targetDate);
    const targetPageId = result.page.id;
    const targetItems = result.items;

    const groupIds = this.state.getItemWithChildren(item.id).map(b => b.id);

    const position: ItemPosition = direction === 'next' ? 'last' : 'first';
    await this.api.moveItems(groupIds, targetPageId, position);

    // If target page had only one item with empty body, delete it
    if (targetItems.length === 1 && targetItems[0].body === '') {
      await this.api.deleteItem(targetItems[0].id);
    }

    await this.keepSourcePageNonEmpty(item.pageId, groupIds);

    // The journal navigates to the next day when the item left the page it shows
    const navigate = direction === 'next' && item.pageId === shownPageId;
    return { targetDate, navigate, itemId: item.id };
  }

  /**
   * Moves a level 2 journal item to an adjacent day's page, if the target page
   * is loaded and has a valid placement position.
   * For 'next': scans from the end of the next day's page.
   * For 'prev': scans from the start of the previous day's page.
   * @returns What the move did, or null when the item was not moved
   */
  private async moveLevel2ToAdjacentDay(item: Item, direction: 'next' | 'prev'): Promise<CrossDayMove | null> {
    const targetDate = this.adjacentDate(item, direction);

    // Only proceed if target day's page is already loaded
    const targetPageItems = this.state.items().filter(b =>
      b.type === 'journal' && b.name === targetDate
    );
    if (targetPageItems.length === 0) return null;

    const scanDirection = direction === 'next' ? 'from-end' : 'from-start';
    const placement = findCrossPagePlacement(targetPageItems, item.level, scanDirection);
    if (!placement) return null;

    const groupIds = this.state.getItemWithChildren(item.id).map(b => b.id);
    const targetPageId = targetPageItems[0].pageId;

    await this.api.moveItems(groupIds, targetPageId, placement);
    await this.keepSourcePageNonEmpty(item.pageId, groupIds);

    return { targetDate, navigate: false, itemId: item.id };
  }

  /** The date of the day before or after a daily journal item. */
  private adjacentDate(item: Item, direction: 'next' | 'prev'): string {
    return shiftIsoDate(item.name, direction === 'next' ? 1 : -1);
  }

  /** A page must keep at least one item: creates an empty one when the moved items were the last ones. */
  private async keepSourcePageNonEmpty(sourcePageId: string, movedIds: string[]): Promise<void> {
    const remaining = this.state.items().filter(b => b.pageId === sourcePageId && !movedIds.includes(b.id));
    if (remaining.length === 0) {
      await this.api.createItem(sourcePageId, 'first', 1, '');
    }
  }
}
