import assert from 'node:assert/strict';
import { test } from 'node:test';
import { dimensions } from '../image.mjs';

test('without createImageBitmap the probe yields null', async () => {
  const saved = globalThis.createImageBitmap;
  delete globalThis.createImageBitmap;
  assert.equal(await dimensions(new Uint8Array([1, 2])), null);
  globalThis.createImageBitmap = saved;
});

test('bitmap dimensions are read and the bitmap closed', async () => {
  let closed = false;
  globalThis.createImageBitmap = async () => ({ width: 3, height: 4, close: () => (closed = true) });
  assert.deepEqual(await dimensions(new Uint8Array([1])), { width: 3, height: 4 });
  assert.equal(closed, true);
  globalThis.createImageBitmap = async () => {
    throw new Error('decode');
  };
  assert.equal(await dimensions(new Uint8Array([1])), null);
  delete globalThis.createImageBitmap;
});
