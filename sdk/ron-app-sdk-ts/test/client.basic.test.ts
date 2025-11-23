import { describe, it, expect } from 'vitest';
import { Ron } from '../src/client';

describe('Ron client (basic scaffold)', () => {
  it('constructs with baseUrl', () => {
    const ron = new Ron({ baseUrl: 'https://example.com' });
    expect(ron).toBeTruthy();
  });
});
