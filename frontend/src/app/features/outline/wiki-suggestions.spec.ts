import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { item } from './outline-test-support';
import { WikiSuggestionPick, WikiSuggestionsComponent } from './wiki-suggestions';

describe('WikiSuggestionsComponent', () => {
  let searchPageNames: ReturnType<typeof vi.fn>;
  let popup: WikiSuggestionsComponent;
  let picks: WikiSuggestionPick[];

  const rect = { left: 10, bottom: 30 } as DOMRect;
  const typed = (query: string) => popup.query(item('a', 1), { query, rect });
  const searched = async () => {
    await vi.advanceTimersByTimeAsync(WikiSuggestionsComponent.SEARCH_DELAY_MS);
  };

  beforeEach(() => {
    vi.useFakeTimers();
    searchPageNames = vi.fn(async (query: string) => ({
      results: [{ pageTitle: query + ' 1' }, { pageTitle: query + ' 2' }],
    }));
    TestBed.configureTestingModule({ providers: [{ provide: ApiService, useValue: { searchPageNames } }] });
    popup = TestBed.createComponent(WikiSuggestionsComponent).componentInstance;
    picks = [];
    popup.picked.subscribe(pick => picks.push(pick));
  });

  afterEach(() => vi.useRealTimers());

  it('searches page names once typing pauses and opens below the caret', async () => {
    typed('pe');
    typed('pet');
    expect(searchPageNames).not.toHaveBeenCalled();

    await searched();

    expect(searchPageNames).toHaveBeenCalledTimes(1);
    expect(searchPageNames).toHaveBeenCalledWith('pet', 10);
    expect(popup.suggestions()).toEqual(['pet 1', 'pet 2']);
    expect(popup.position()).toEqual({ x: 10, y: 34 });
    expect(popup.isShownFor('a')).toBe(true);
    expect(popup.isShownFor('b')).toBe(false);
  });

  it('shows nothing for an empty query', async () => {
    typed('');
    await searched();

    expect(searchPageNames).not.toHaveBeenCalled();
    expect(popup.isShownFor('a')).toBe(false);
  });

  it('closes when the caret leaves the link, and drops a pending search', async () => {
    typed('pet');

    popup.query(item('a', 1), null);
    await searched();

    expect(searchPageNames).not.toHaveBeenCalled();
    expect(popup.isShownFor('a')).toBe(false);
  });

  it('arrow keys move the highlight around the list and Enter picks it', async () => {
    typed('pet');
    await searched();

    popup.keyNav('ArrowUp');
    expect(popup.index()).toBe(1);
    popup.keyNav('ArrowDown');
    expect(popup.index()).toBe(0);
    popup.keyNav('ArrowDown');
    popup.keyNav('Enter');

    expect(picks).toEqual([{ itemId: 'a', title: 'pet 2' }]);
    expect(popup.isShownFor('a')).toBe(false);
  });

  it('a click picks without blurring the editor', async () => {
    typed('pet');
    await searched();
    const event = { preventDefault: vi.fn(), stopPropagation: vi.fn() } as unknown as MouseEvent;

    popup.pick('pet 1', event);

    expect(picks).toEqual([{ itemId: 'a', title: 'pet 1' }]);
    expect(event.preventDefault).toHaveBeenCalled();
  });

  it('a failed search leaves the popup closed', async () => {
    searchPageNames.mockRejectedValueOnce(new Error('offline'));
    typed('pet');
    await searched();

    expect(popup.isShownFor('a')).toBe(false);
  });
});
