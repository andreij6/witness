export const META = 'witness-config';

export const PRODUCTION = Object.freeze({
  host: 'https://icp-api.io',
  registry: '',
  canister: 'https://*.icp0.io',
  local: false,
  anchor: '',
});

const LOCAL_HOSTS = /^(localhost|127\.0\.0\.1|\[::1\]|[a-z0-9-]+\.localhost)$/;

const origin = (text) => {
  try {
    const u = new URL(text);
    return (u.protocol === 'https:' || u.protocol === 'http:') && u.origin === text.replace(/\/$/, '') ? u : null;
  } catch {
    return null;
  }
};

export function isLocalHost(host) {
  const u = origin(host);
  return u !== null && LOCAL_HOSTS.test(u.hostname);
}

export function parseConfig(text) {
  let raw;
  try {
    raw = text ? JSON.parse(text) : {};
  } catch {
    raw = {};
  }
  if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) raw = {};
  const pick = (k, ok) => (ok(raw[k]) ? raw[k] : PRODUCTION[k]);
  const host = pick('host', (v) => typeof v === 'string' && origin(v) !== null).replace(/\/$/, '');
  const canister = pick('canister', (v) => typeof v === 'string' && v.split('*').length === 2 && origin(v.replace('*', 'x')) !== null);
  return Object.freeze({
    host,
    canister,
    registry: pick('registry', (v) => typeof v === 'string' && /^[a-z0-9-]{5,63}$/.test(v)),
    anchor: pick('anchor', (v) => typeof v === 'string' && /^([0-9a-f]{2})*$/.test(v)),
    local: raw.local === true && isLocalHost(host),
  });
}

export function readConfig(doc) {
  const meta = doc?.querySelector?.(`meta[name="${META}"]`);
  return parseConfig(meta?.getAttribute('content'));
}

export const canisterUrl = (config, id, path) => config.canister.replace('*', id) + path;
