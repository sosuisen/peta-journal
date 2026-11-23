import { renderHook } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { useAsked } from './askedContext';
import { useCore } from './context';

// コアと画面の操作の文脈
describe('the contexts of the core and of the operations', () => {
  // コアの文脈の外でコアを使うと、エラーになる。
  test('the core outside its context is an error', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => renderHook(() => useCore())).toThrow(Error);
    error.mockRestore();
  });

  // 画面の操作の文脈の外で画面の操作を使うと、エラーになる。
  test('the operations of the screen outside their context are an error', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => renderHook(() => useAsked())).toThrow(Error);
    error.mockRestore();
  });
});
