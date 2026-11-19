export const KEY_BYTES = 32;
export const NONCE_BYTES = 12;
export const TAG_BITS = 128;

const subtle = () => globalThis.crypto.subtle;

const importKey = (raw, usage) => subtle().importKey('raw', raw, { name: 'AES-GCM' }, false, [usage]);

const params = (nonce, aad) => ({ name: 'AES-GCM', iv: nonce, additionalData: aad, tagLength: TAG_BITS });

export async function seal(key, nonce, plaintext, aad) {
  const k = await importKey(key, 'encrypt');
  return new Uint8Array(await subtle().encrypt(params(nonce, aad), k, plaintext));
}

export async function open(key, nonce, ciphertext, aad) {
  try {
    const k = await importKey(key, 'decrypt');
    return new Uint8Array(await subtle().decrypt(params(nonce, aad), k, ciphertext));
  } catch {
    return null;
  }
}

export async function sha256(data) {
  return new Uint8Array(await subtle().digest('SHA-256', data));
}

export function randomBytes(n) {
  return globalThis.crypto.getRandomValues(new Uint8Array(n));
}

export async function sealChunk(key, nonce, plaintext, aad) {
  const ct = await seal(key, nonce, plaintext, aad);
  const out = new Uint8Array(nonce.length + ct.length);
  out.set(nonce);
  out.set(ct, nonce.length);
  return out;
}

export async function openChunk(key, chunk, aad) {
  if (chunk.length < NONCE_BYTES + TAG_BITS / 8) return null;
  return open(key, chunk.subarray(0, NONCE_BYTES), chunk.subarray(NONCE_BYTES), aad);
}
