import config from './witness-config.mjs';

const WASM_GC_PROBE = new Uint8Array([
  0, 97, 115, 109, 1, 0, 0, 0, 1, 8, 2, 95, 1, 120, 0, 96, 0, 0, 3, 3, 2, 1, 1, 10,
  14, 2, 6, 0, 6, 64, 25, 11, 11, 5, 0, 208, 112, 26, 11, 0, 45, 4, 110, 97, 109, 101, 1, 15,
  2, 0, 5, 102, 117, 110, 99, 48, 1, 5, 102, 117, 110, 99, 49, 4, 8, 1, 0, 5, 116, 121, 112, 101,
  48, 10, 11, 1, 0, 1, 0, 6, 102, 105, 101, 108, 100, 48,
]);

const pinMap = document.querySelector('script[type="importmap"]');
const pinned = pinMap ? (JSON.parse(pinMap.textContent).integrity ?? {}) : null;
const base = new URL('./', import.meta.url);
const CODE = /\.(?:m?js|wasm)$/;

if (pinned) {
  const native = globalThis.fetch.bind(globalThis);
  globalThis.fetch = (input, init) => {
    const url = new URL(input instanceof Request ? input.url : String(input), document.baseURI);
    const integrity = pinned[url.href];
    if (integrity) return native(input, { ...init, integrity });
    if (CODE.test(url.pathname)) return Promise.reject(new TypeError(`witness: ${url.href} is not pinned`));
    return native(input, init);
  };
}

const wasmGc = () => typeof WebAssembly?.validate === 'function' && WebAssembly.validate(WASM_GC_PROBE);

if (!document.querySelector('meta[name="witness-resources"]')) {
  const meta = document.createElement('meta');
  meta.name = 'witness-resources';
  meta.content = base.href;
  document.head.append(meta);
}

if (config && !document.querySelector('meta[name="witness-config"]')) {
  const meta = document.createElement('meta');
  meta.name = 'witness-config';
  meta.content = JSON.stringify(config);
  document.head.append(meta);
}

const style = document.createElement('style');
style.textContent = '#app{position:fixed;inset:0}body.compose-ready>main{display:none}';
document.head.append(style);

const load = (name, fallback) => {
  const script = document.createElement('script');
  script.src = new URL(name, base).href;
  if (pinned) {
    const integrity = pinned[script.src];
    if (!integrity) throw new TypeError(`witness: ${script.src} is not pinned`);
    script.integrity = integrity;
    script.crossOrigin = 'anonymous';
  } else if (fallback) {
    script.onerror = () => load(fallback);
  }
  document.body.append(script);
};

load(wasmGc() ? 'originWasmWitness.js' : 'originJsWitness.js', 'witness.js');
