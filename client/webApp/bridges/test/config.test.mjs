import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PRODUCTION, canisterUrl, isLocalHost, parseConfig, readConfig } from '../config.mjs';

const LOCAL = { host: 'http://localhost:8090', registry: '4caro-hl777-77775-aaaba-cai', canister: 'http://*.localhost:8090', local: true };

test('missing or malformed config falls back to production', () => {
  for (const text of [null, '', '{', '[]', 'null', '42', '"x"']) assert.deepEqual({ ...parseConfig(text) }, { ...PRODUCTION });
});

test('local config is accepted for a local gateway', () => {
  const c = parseConfig(JSON.stringify(LOCAL));
  assert.equal(c.host, 'http://localhost:8090');
  assert.equal(c.local, true);
  assert.equal(c.registry, LOCAL.registry);
  assert.equal(canisterUrl(c, 'abc', '/p/X'), 'http://abc.localhost:8090/p/X');
});

test('fetching the root key is refused for non-local hosts', () => {
  assert.equal(parseConfig(JSON.stringify({ ...LOCAL, host: 'https://icp-api.io' })).local, false);
  assert.equal(parseConfig(JSON.stringify({ ...LOCAL, local: 'true' })).local, false);
  assert.equal(isLocalHost('http://127.0.0.1:4943'), true);
  assert.equal(isLocalHost('http://abc.localhost:8090'), true);
  assert.equal(isLocalHost('http://localhost.evil.com'), false);
  assert.equal(isLocalHost('not a url'), false);
});

test('invalid fields fall back field by field', () => {
  const c = parseConfig(JSON.stringify({ host: 'ftp://x', registry: 'BAD ID', canister: 'https://no-wildcard', anchor: 'zz' }));
  assert.equal(c.host, PRODUCTION.host);
  assert.equal(c.registry, PRODUCTION.registry);
  assert.equal(c.canister, PRODUCTION.canister);
  assert.equal(c.anchor, PRODUCTION.anchor);
  assert.equal(parseConfig(JSON.stringify({ canister: 'https://*.*.x' })).canister, PRODUCTION.canister);
  assert.equal(parseConfig(JSON.stringify({ host: 'http://localhost:8090/' })).host, 'http://localhost:8090');
  assert.equal(parseConfig(JSON.stringify({ host: 'http://localhost:8090/api' })).host, PRODUCTION.host);
});

test('readConfig reads the meta tag', () => {
  const doc = { querySelector: (q) => (q === 'meta[name="witness-config"]' ? { getAttribute: () => JSON.stringify(LOCAL) } : null) };
  assert.equal(readConfig(doc).registry, LOCAL.registry);
  assert.deepEqual({ ...readConfig({ querySelector: () => null }) }, { ...PRODUCTION });
  assert.deepEqual({ ...readConfig(undefined) }, { ...PRODUCTION });
});
