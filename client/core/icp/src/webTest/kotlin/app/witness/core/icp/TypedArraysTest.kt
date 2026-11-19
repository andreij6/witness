package app.witness.core.icp

import app.witness.core.icp.web.toByteArray
import app.witness.core.icp.web.toJsInt32Array
import app.witness.core.icp.web.toKotlinIntArray
import app.witness.core.icp.web.toUint8Array
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

class TypedArraysTest {
    @Test
    fun bytesRoundTripPreservingSign() {
        val bytes = ByteArray(256) { it.toByte() }
        val u8 = bytes.toUint8Array()
        assertEquals(256, u8.length)
        assertContentEquals(bytes, u8.toByteArray())
    }

    @Test
    fun emptyArrays() {
        assertContentEquals(ByteArray(0), ByteArray(0).toUint8Array().toByteArray())
        assertContentEquals(IntArray(0), IntArray(0).toJsInt32Array().toKotlinIntArray())
    }

    @Test
    fun intsRoundTrip() {
        val ints = intArrayOf(0, -1, Int.MIN_VALUE, Int.MAX_VALUE, 42)
        assertContentEquals(ints, ints.toJsInt32Array().toKotlinIntArray())
    }
}
