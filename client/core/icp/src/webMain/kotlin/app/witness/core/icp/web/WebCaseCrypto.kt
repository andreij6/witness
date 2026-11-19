package app.witness.core.icp.web

import app.witness.core.icp.cases.CaseCrypto
import app.witness.core.icp.web.openChunk as jsOpenChunk
import app.witness.core.icp.web.randomBytes as jsRandomBytes
import app.witness.core.icp.web.sealChunk as jsSealChunk
import app.witness.core.icp.web.sha256 as jsSha256
import kotlinx.coroutines.await
import org.khronos.webgl.Uint8Array

class WebCaseCrypto : CaseCrypto {
    override suspend fun sealChunk(key: ByteArray, nonce: ByteArray, plaintext: ByteArray, from: Int, to: Int, aad: ByteArray): ByteArray =
        jsSealChunk(key.toUint8Array(), nonce.toUint8Array(), plaintext.copyOfRange(from, to).toUint8Array(), aad.toUint8Array())
            .await<Uint8Array>().toByteArray()

    override suspend fun openChunk(key: ByteArray, chunk: ByteArray, aad: ByteArray): ByteArray? =
        jsOpenChunk(key.toUint8Array(), chunk.toUint8Array(), aad.toUint8Array()).await<Uint8Array?>()?.toByteArray()

    override suspend fun sha256(data: ByteArray): ByteArray = jsSha256(data.toUint8Array()).await<Uint8Array>().toByteArray()

    override fun randomBytes(count: Int): ByteArray = jsRandomBytes(count).toByteArray()
}
