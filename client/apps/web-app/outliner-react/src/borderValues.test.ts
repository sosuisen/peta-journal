import { expect, test } from 'vitest';
import examples from '@common-tests/border-web.json?raw';
import { Changes, fromJsonEvent, toJsonEvent } from './border/peta-types';
import * as json from './border/serde/json';

/** An example of the core: a value of the border, as the text of JSON that crosses it. */
type Example = { id: string; json: string };

type Part = 'changes' | 'events';

/** The examples of a part of the file. */
function part(name: Part): Example[] {
  return (JSON.parse(examples) as Record<Part, Example[]>)[name];
}

/**
 * `written`, with each number that is the same float as a bigint of `read` in its place.
 *
 * The core writes a float with all its digits, as `f64::MAX` is, and `json.parse` reads such
 * digits as a bigint. The shell writes the float back as JavaScript writes it, with an exponent,
 * and the core reads that as the same float. An integer that lost its digits in the shell is
 * written as digits again, and read as another bigint: it stays apart.
 */
function asFloatsWereRead(written: unknown, read: unknown): unknown {
  if (typeof read === 'bigint' && typeof written === 'number') {
    return Number(read) === written ? read : written;
  }
  if (Array.isArray(read) && Array.isArray(written)) {
    return written.map((value, index) => asFloatsWereRead(value, read[index]));
  }
  if (isObject(read) && isObject(written)) {
    return Object.fromEntries(
      Object.entries(written).map(([key, value]) => [
        key,
        asFloatsWereRead(value, read[key]),
      ]),
    );
  }
  return written;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Reads each example as the shell reads the border, writes it back as the shell writes it,
 * and expects the value it began with. Returns the ids of the examples it ran.
 */
function readAndWriteBack(
  name: Part,
  readAndWrite: (value: unknown) => json.JsonValue,
): string[] {
  const ran: string[] = [];
  for (const example of part(name)) {
    const value = json.parse(example.json);

    const written = json.stringify(readAndWrite(value));

    expect(asFloatsWereRead(json.parse(written), value), example.id).toEqual(
      value,
    );
    ran.push(example.id);
  }
  return ran;
}

// コアの`Changes`の例（ADR 090のD2）を、Webのシェルの生成された型で読んで書き直すと、元の値に戻る。回した例は、例のファイルのすべてである。
test('the shell reads and writes back every example of the changes as it was', () => {
  const ran = readAndWriteBack('changes', value =>
    Changes.toJson(Changes.fromJson(value)),
  );

  expect(ran.length).toBeGreaterThan(0);
  expect(ran).toEqual(part('changes').map(example => example.id));
});

// コアの`Event`の例を、Webのシェルの生成された型で読んで書き直すと、元の値に戻る。回した例は、例のファイルのすべてである。
test('the shell reads and writes back every example of the events as it was', () => {
  const ran = readAndWriteBack('events', value =>
    toJsonEvent(fromJsonEvent(value)),
  );

  expect(ran.length).toBeGreaterThan(0);
  expect(ran).toEqual(part('events').map(example => example.id));
});
