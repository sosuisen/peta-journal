import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiService } from '../../core/services/api.service';
import { OutlineStateService } from '../../core/services/outline-state.service';
import { Item } from '../../core/models/item.model';
import { JournalDayMoveService } from './journal-day-move.service';
import { fakeApi, item } from './outline-test-support';

describe('JournalDayMoveService', () => {
  let state: OutlineStateService;
  let api: ReturnType<typeof fakeApi> & { getOrCreateJournal: ReturnType<typeof vi.fn> };
  let service: JournalDayMoveService;

  const journalItem = (id: string, level: number, date: string, extra: Partial<Item> = {}) =>
    item(id, level, { pageId: 'page-' + date, type: 'journal', name: date, ...extra });

  // The journal shows the newest day first:
  //   2026-03-02:  n1 (1)
  //   2026-03-01:  t1 (1) > t2 (2), t3 (2)
  beforeEach(() => {
    state = new OutlineStateService();
    state.setItems([
      journalItem('n1', 1, '2026-03-02'),
      journalItem('t1', 1, '2026-03-01'),
      journalItem('t2', 2, '2026-03-01'),
      journalItem('t3', 2, '2026-03-01'),
    ]);
    api = {
      ...fakeApi(),
      getOrCreateJournal: vi.fn(async (date: string) => ({ page: { id: 'page-' + date }, items: [], created: true })),
    };
    service = new JournalDayMoveService(state, api as unknown as ApiService);
  });

  describe('a level 1 item', () => {
    it('moving the topmost item up takes it with its children to the end of the next day', async () => {
      const result = await service.moveAcrossDays(state.getItemById('t1')!, 'up', 'page-2026-03-01');

      expect(api.getOrCreateJournal).toHaveBeenCalledWith('2026-03-02');
      expect(api.moveItems).toHaveBeenCalledWith(['t1', 't2', 't3'], 'page-2026-03-02', 'last');
      expect(result).toEqual({ targetDate: '2026-03-02', navigate: true, itemId: 't1' });
    });

    it('does not navigate when the item was not on the shown page', async () => {
      const result = await service.moveAcrossDays(state.getItemById('t1')!, 'up', 'page-2026-03-02');

      expect(result?.navigate).toBe(false);
    });

    it('moving the bottommost item down takes it to the start of the previous day', async () => {
      const result = await service.moveAcrossDays(state.getItemById('n1')!, 'down', 'page-2026-03-02');

      expect(api.getOrCreateJournal).toHaveBeenCalledWith('2026-03-01');
      expect(api.moveItems).toHaveBeenCalledWith(['n1'], 'page-2026-03-01', 'first');
      expect(result).toEqual({ targetDate: '2026-03-01', navigate: false, itemId: 'n1' });
    });

    it('replaces the single empty item of a new target page', async () => {
      api.getOrCreateJournal.mockResolvedValueOnce({ page: { id: 'page-2026-03-03' }, items: [item('empty', 1, { body: '' })] });

      await service.moveAcrossDays(state.getItemById('n1')!, 'up', 'page-2026-03-02');

      expect(api.deleteItem).toHaveBeenCalledWith('empty');
    });

    it('leaves an empty item behind when the source page would become empty', async () => {
      await service.moveAcrossDays(state.getItemById('n1')!, 'up', 'page-2026-03-02');

      expect(api.createItem).toHaveBeenCalledWith('page-2026-03-02', 'first', 1, '');
    });

    it('an item in the middle of its page stays', async () => {
      state.setItems([journalItem('a', 1, '2026-03-01'), journalItem('b', 1, '2026-03-01'), journalItem('c', 1, '2026-03-01')]);

      expect(await service.moveAcrossDays(state.getItemById('b')!, 'up', 'page-2026-03-01')).toBeNull();
      expect(await service.moveAcrossDays(state.getItemById('b')!, 'down', 'page-2026-03-01')).toBeNull();
      expect(api.moveItems).not.toHaveBeenCalled();
    });
  });

  describe('a level 2 item', () => {
    it('in second position moves up under the last root of the loaded next day', async () => {
      const result = await service.moveAcrossDays(state.getItemById('t2')!, 'up', 'page-2026-03-01');

      expect(api.getOrCreateJournal).not.toHaveBeenCalled();
      expect(api.moveItems).toHaveBeenCalledWith(['t2'], 'page-2026-03-02', 'after:n1');
      expect(result).toEqual({ targetDate: '2026-03-02', navigate: false, itemId: 't2' });
    });

    it('stays when the adjacent day is not loaded', async () => {
      const result = await service.moveAcrossDays(state.getItemById('t3')!, 'down', 'page-2026-03-01');

      expect(result).toBeNull();
      expect(api.moveItems).not.toHaveBeenCalled();
    });
  });
});
