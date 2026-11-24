import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import Frame from './Frame';

/** A part of the app that fails to draw. */
function Broken(): never {
  throw new Error('broken');
}

// React logs each error that a boundary catches
beforeEach(() => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

// アプリのフレーム
describe('a frame of the app', () => {
  // フレームの中で例外が出ると、そのフレームは描けなかったことを表示する。
  test('a frame whose part fails tells that it could not be drawn', () => {
    render(
      <Frame>
        <Broken />
      </Frame>,
    );

    expect(screen.queryByRole('alert')).not.toBeNull();
  });
});
