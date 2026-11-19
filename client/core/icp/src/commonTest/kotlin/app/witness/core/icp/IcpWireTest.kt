package app.witness.core.icp

import app.witness.core.icp.registry.ShardInfo
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class IcpWireTest {
    @Test
    fun u64ParsesDecimal() {
        assertEquals(0, IcpWire.u64("0"))
        assertEquals(42, IcpWire.u64("42"))
        assertEquals(Long.MAX_VALUE, IcpWire.u64("9223372036854775807"))
        assertEquals(7, IcpWire.u64("0000007"))
    }

    @Test
    fun u64RejectsMalformedAndOverflow() {
        for (bad in listOf("", "-1", "+1", " 1", "1 ", "1.0", "0x10", "9223372036854775808", "18446744073709551615", "12345678901234567890", "١")) {
            assertFailsWith<IcpException>(bad) { IcpWire.u64(bad) }
        }
    }

    @Test
    fun recordIdsSplitAndValidate() {
        assertEquals(emptyList(), IcpWire.recordIds(""))
        val ids = IcpWire.recordIds("WERC8GMRZGE196QV,0000000000000000")
        assertEquals(listOf("WERC8GMRZGE196QV", "0000000000000000"), ids.map { it.text })
        assertFailsWith<IcpException> { IcpWire.recordIds("WERC8GMRZGE196QV,") }
        assertFailsWith<IcpException> { IcpWire.recordIds("nope") }
    }

    @Test
    fun recordIdCanonicalises() {
        assertEquals("WERC8GMRZGE196QV", IcpWire.recordId("werc-8gmr-zge1-96qv").text)
    }

    @Test
    fun shardValidatesFields() {
        assertEquals(ShardInfo("aaaaa-aa", 10, 20), IcpWire.shard("aaaaa-aa", "10", "20"))
        assertFailsWith<IcpException> { IcpWire.shard("", "1", "2") }
        assertFailsWith<IcpException> { IcpWire.shard("a,b", "1", "2") }
        assertFailsWith<IcpException> { IcpWire.shard("a", "-1", "2") }
    }

    @Test
    fun joinIdsKeepsOrder() {
        assertEquals("a,b,c", IcpWire.joinIds(listOf(ShardInfo("a", 0, 0), ShardInfo("b", 1, 0), ShardInfo("c", 2, 0))))
        assertEquals("", IcpWire.joinIds(emptyList()))
    }
}
