package app.witness.core.icp.web

import org.khronos.webgl.Int32Array
import org.khronos.webgl.Int8Array
import org.khronos.webgl.Uint8Array
import org.khronos.webgl.get
import org.khronos.webgl.toByteArray
import org.khronos.webgl.toInt32Array
import org.khronos.webgl.toInt8Array

fun ByteArray.toUint8Array(): Uint8Array {
    val signed = toInt8Array()
    return Uint8Array(signed.buffer, signed.byteOffset, signed.length)
}

fun Uint8Array.toByteArray(): ByteArray = Int8Array(buffer, byteOffset, length).toByteArray()

fun IntArray.toJsInt32Array(): Int32Array = toInt32Array()

fun Int32Array.toKotlinIntArray(): IntArray = IntArray(length) { this[it] }
