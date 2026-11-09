import { describe, expect, test } from 'vitest';
import type { Item } from './api';
import {
  type Filters,
  getDue,
  getPlan,
  groupByPlan,
  isShown,
  pagePath,
  taskLabel,
} from './tasks';

/** A task on the doc page `Notes`, changed by `overrides`. */
function item(overrides: Partial<Item> = {}): Item {
  return {
    id: 'item-1',
    pageId: 'page-1',
    dirPath: '',
    name: 'Notes',
    type: 'doc',
    createdDate: 0,
    modifiedDate: 0,
    idx: 'a0',
    level: 1,
    body: 'a task',
    ast: '{}',
    collapsed: false,
    types: ['task'],
    properties: {},
    ...overrides,
  };
}

/** A task with these properties. */
function withProperties(properties: Item['properties']): Item {
  return item({ properties });
}

// タスクの状態のラベル
describe('the label of a task', () => {
  // 開始も終了もないタスクは、TODOである。
  test('is TODO without a begin and an end', () => {
    expect(taskLabel(withProperties({}))).toBe('TODO');
  });

  // 開始があって終了のないタスクは、RUNNINGである。
  test('is RUNNING with a begin and no end', () => {
    expect(taskLabel(withProperties({ begin: '2026-11-09T10:00' }))).toBe(
      'RUNNING',
    );
  });

  // 終了のあるタスクは、開始があってもDONEである。
  test('is DONE with an end, also when it has a begin', () => {
    const done = withProperties({
      begin: '2026-11-09T10:00',
      end: '2026-11-09T11:00',
    });

    expect(taskLabel(done)).toBe('DONE');
  });
});

// 開始日と期限
describe('the plan date and the due date', () => {
  // 開始日は`plan`プロパティ、期限は`due`プロパティである。
  test('are the plan and the due property', () => {
    const task = withProperties({ plan: '2026-11-03', due: '2026-12-24' });

    expect(getPlan(task)).toBe('2026-11-03');
    expect(getDue(task)).toBe('2026-12-24');
  });

  // プロパティがなければ、開始日も期限もない。
  test('are missing without the properties', () => {
    expect(getPlan(withProperties({}))).toBeNull();
    expect(getDue(withProperties({}))).toBeNull();
  });

  // 文字列でない値は、日付として扱わない。
  test('are missing when the value is not a string', () => {
    const task = withProperties({ plan: ['2026-11-03'], due: ['2026-12-24'] });

    expect(getPlan(task)).toBeNull();
    expect(getDue(task)).toBeNull();
  });
});

// 絞り込み
describe('the filters', () => {
  const all: Filters = {
    untilDate: '',
    showDone: true,
    showJournal: true,
    showPage: true,
  };
  const onJournal = item({ name: '2026-11-09' });
  const onPage = item({ name: 'Notes' });

  // 完了したタスクは、Doneを選んだときだけ表示される。
  test('show a finished task only with Done', () => {
    const done = withProperties({ end: '2026-11-09T11:00' });

    expect(isShown(done, all)).toBe(true);
    expect(isShown(done, { ...all, showDone: false })).toBe(false);
  });

  // 完了していないタスクは、Doneを選ばなくても表示される。
  test('show an open task without Done', () => {
    expect(isShown(onPage, { ...all, showDone: false })).toBe(true);
  });

  // 日次ジャーナルのタスクは、Journalを選んだときだけ表示される。
  test('show a task of a daily journal only with Journal', () => {
    expect(isShown(onJournal, { ...all, showPage: false })).toBe(true);
    expect(isShown(onJournal, { ...all, showJournal: false })).toBe(false);
  });

  // 日次ジャーナル以外のページのタスクは、Pageを選んだときだけ表示される。
  test('show a task of another page only with Page', () => {
    expect(isShown(onPage, { ...all, showJournal: false })).toBe(true);
    expect(isShown(onPage, { ...all, showPage: false })).toBe(false);
  });

  // 週、月、年のジャーナルのタスクも、Journalを選んだときだけ表示される。
  test('show a task of a weekly, monthly or yearly journal only with Journal', () => {
    for (const name of ['2026-W45', '2026-11', '2026']) {
      const task = item({ type: 'journal', name });

      expect(isShown(task, { ...all, showPage: false }), name).toBe(true);
      expect(isShown(task, { ...all, showJournal: false }), name).toBe(false);
    }
  });

  // 開始日が指定の日より後のタスクは、表示されない。指定の日のタスクは表示される。
  test('hide a task that starts after the given date', () => {
    const until = { ...all, untilDate: '2026-11-05' };

    expect(isShown(withProperties({ plan: '2026-11-04' }), until)).toBe(true);
    expect(isShown(withProperties({ plan: '2026-11-05' }), until)).toBe(true);
    expect(isShown(withProperties({ plan: '2026-11-06' }), until)).toBe(false);
  });

  // 開始日のないタスクは、日を指定しても表示される。
  test('show a task without a plan date whatever the given date', () => {
    expect(isShown(onPage, { ...all, untilDate: '2026-11-05' })).toBe(true);
  });

  // 日を指定しなければ、開始日が先のタスクも表示される。
  test('show every plan date when no date is given', () => {
    expect(isShown(withProperties({ plan: '2099-01-01' }), all)).toBe(true);
  });
});

// 開始日ごとのまとめ
describe('grouping by plan date', () => {
  const task = (id: string, plan?: string) =>
    item({ id, properties: plan ? { plan } : {} });
  /** The entries as short strings: `# heading` or the id of the item. */
  const outline = (items: Item[]) =>
    groupByPlan(items).map(entry =>
      entry.kind === 'heading' ? `# ${entry.label}` : entry.item.id,
    );

  // タスクは開始日の早い順に並び、日付ごとに見出しが付く。
  test('sorts the tasks by plan date under a heading for each date', () => {
    const items = [task('c', '2026-11-03'), task('a', '2026-11-01')];

    expect(outline(items)).toEqual(['# 2026-11-01', 'a', '# 2026-11-03', 'c']);
  });

  // 同じ開始日のタスクは、1つの見出しの下に、元の順で並ぶ。
  test('keeps tasks of one date under one heading in their order', () => {
    const items = [
      task('b', '2026-11-03'),
      task('a', '2026-11-01'),
      task('c', '2026-11-03'),
    ];

    expect(outline(items)).toEqual([
      '# 2026-11-01',
      'a',
      '# 2026-11-03',
      'b',
      'c',
    ]);
  });

  // 開始日のないタスクは、最後に「開始日なし」の見出しの下に、元の順で並ぶ。
  test('puts tasks without a plan date last under their own heading', () => {
    const items = [task('x'), task('a', '2026-11-01'), task('y')];

    expect(outline(items)).toEqual([
      '# 2026-11-01',
      'a',
      '# 開始日なし',
      'x',
      'y',
    ]);
  });

  // 開始日のないタスクがなければ、「開始日なし」の見出しは付かない。
  test('has no heading for missing plan dates when every task has one', () => {
    expect(outline([task('a', '2026-11-01')])).toEqual(['# 2026-11-01', 'a']);
  });

  // タスクがなければ、見出しもない。
  test('is empty without tasks', () => {
    expect(outline([])).toEqual([]);
  });
});

// PetaJournalでページを開くパス
describe('the path that opens the page of a task', () => {
  // ドキュメントページは`/docs/`の下で、項目のIDがフラグメントに付く。
  test('is under /docs/ for a doc page, with the item id as the fragment', () => {
    const task = item({ id: 'item-7', name: 'Todo Check' });

    expect(pagePath(task)).toBe('/docs/Todo Check#item-7');
  });

  // フォルダの中のページは、フォルダのパスがページ名の前に付く。
  test('has the folder before the name of a page in a folder', () => {
    const task = item({ id: 'item-7', dirPath: 'proj/', name: 'Plan' });

    expect(pagePath(task)).toBe('/docs/proj/Plan#item-7');
  });

  // 日、週、月、年のジャーナルは`/journals/`の下である。
  test('is under /journals/ for a daily, weekly, monthly or yearly journal', () => {
    for (const name of [
      '2026-11-09',
      '2026-W45',
      '2026-W5',
      '2026-11',
      '2026',
    ]) {
      const task = item({ id: 'item-7', name });

      expect(pagePath(task)).toBe(`/journals/${name}#item-7`);
    }
  });

  // 日付で始まるだけの名前のページは、ドキュメントページである。
  test('is under /docs/ for a page whose name only starts with a date', () => {
    const task = item({ id: 'item-7', name: '2026-11-09 memo' });

    expect(pagePath(task)).toBe('/docs/2026-11-09 memo#item-7');
  });
});
