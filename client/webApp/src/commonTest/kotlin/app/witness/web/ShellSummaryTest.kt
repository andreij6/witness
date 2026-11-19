package app.witness.web

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ShellSummaryTest {
    @Test
    fun readsRecordAndShard() {
        assertEquals(ShellSummary("ABC123", 3), ShellSummary.parse("""{"record_id":"ABC123","media_shard":3}"""))
    }

    @Test
    fun ignoresUnknownFields() {
        assertEquals(ShellSummary("A", null), ShellSummary.parse("""{"record_id":"A","extra":[1,2]}"""))
    }

    @Test
    fun emptyOrMalformedIsNull() {
        listOf(null, "", "   ", "not json", "[1,2]", "\"x\"", "{").forEach { assertNull(ShellSummary.parse(it), it.toString()) }
    }

    @Test
    fun rejectsHostileOrWrongTypedValues() {
        assertEquals(ShellSummary(null, null), ShellSummary.parse("""{"record_id":"../etc","media_shard":"3"}"""))
        assertEquals(ShellSummary(null, null), ShellSummary.parse("""{"record_id":12,"media_shard":-1}"""))
        assertEquals(ShellSummary(null, null), ShellSummary.parse("""{"record_id":"<b>","media_shard":1.5}"""))
    }

    @Test
    fun oversizedInputIsRejected() {
        assertNull(ShellSummary.parse("""{"record_id":"A","pad":"${"x".repeat(ShellSummary.MAX_BYTES)}"}"""))
    }
}
