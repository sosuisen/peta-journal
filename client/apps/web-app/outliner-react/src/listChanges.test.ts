import { expect, test } from 'vitest';
import examples from '@common-tests/list-changes.json?raw';
import { fromJsonListChange, Row } from './border/peta-types';
import * as json from './border/serde/json';
import { rowsAfter } from './screen';

/** An example of the core: the rows before, their changes and the rows after, as JSON. */
type Example = {
  id: string;
  before: unknown[];
  changes: unknown[];
  after: unknown[];
};

/** The examples of the rows for the web shell, read as the shell reads the border. */
function rowExamples(): Example[] {
  const whole = json.parse(examples) as { web: { rows: Example[] } };
  return whole.web.rows;
}

// コアの差分の例（ADR 090のD1）を、Webのシェルが当てると、すべてコアのあとの行になる。回した例は、例のファイルのすべてである。
test('the shell applies every example of the core to the rows as the core does', () => {
  const ran: string[] = [];

  for (const example of rowExamples()) {
    const before = example.before.map(row => Row.fromJson(row));
    const changes = example.changes.map(change => fromJsonListChange(change));
    const after = example.after.map(row => Row.fromJson(row));

    expect(rowsAfter(before, changes), example.id).toEqual(after);
    ran.push(example.id);
  }

  const inFile = (
    JSON.parse(examples) as { web: { rows: { id: string }[] } }
  ).web.rows.map(example => example.id);
  expect(inFile.length).toBeGreaterThan(0);
  expect(ran).toEqual(inFile);
});
