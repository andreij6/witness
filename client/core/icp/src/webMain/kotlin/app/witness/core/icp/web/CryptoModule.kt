@file:JsModule("witness-bridges/crypto.mjs")

package app.witness.core.icp.web

import org.khronos.webgl.Uint8Array
import kotlin.js.Promise

external fun sealChunk(key: Uint8Array, nonce: Uint8Array, plaintext: Uint8Array, aad: Uint8Array): Promise<Uint8Array>

external fun openChunk(key: Uint8Array, chunk: Uint8Array, aad: Uint8Array): Promise<Uint8Array?>

external fun sha256(data: Uint8Array): Promise<Uint8Array>

external fun randomBytes(n: Int): Uint8Array
