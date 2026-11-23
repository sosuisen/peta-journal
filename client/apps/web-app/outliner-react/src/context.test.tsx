import { renderHook } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';
import { useCore } from './context';

// コアの文脈
describe('the context of the core', () => {
  // コアの文脈の外でコアを使うと、エラーになる。
  test('the core outside its context is an error', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => renderHook(() => useCore())).toThrow(Error);
    error.mockRestore();
  });
});
