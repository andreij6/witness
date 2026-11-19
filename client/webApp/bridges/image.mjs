export async function dimensions(bytes) {
  if (typeof globalThis.createImageBitmap !== 'function') return null;
  try {
    const bitmap = await globalThis.createImageBitmap(new Blob([bytes]));
    const out = { width: bitmap.width, height: bitmap.height };
    bitmap.close();
    return out;
  } catch {
    return null;
  }
}
