package app.witness.core.a2ui

import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals

class A2uiMessagesTest {
    @Test
    fun slotPathJoinsUnderSlots() = assertEquals("/slots/summary", A2uiMessages.slotPath("summary"))

    @Test
    fun updateDataModelShape() {
        val message = A2uiMessages.updateDataModel("report", "/slots/summary", JsonPrimitive("hi"))
        assertEquals("v0.9.1", message["version"]?.jsonPrimitive?.content)
        val body = message["updateDataModel"]!!.jsonObject
        assertEquals("report", body["surfaceId"]?.jsonPrimitive?.content)
        assertEquals("/slots/summary", body["path"]?.jsonPrimitive?.content)
        assertEquals(JsonPrimitive("hi"), body["value"])
    }

    @Test
    fun updateDataModelCanClearWithNull() {
        val body = A2uiMessages.updateDataModel("r", "/slots/a", JsonNull)["updateDataModel"]!!.jsonObject
        assertEquals(JsonNull, body["value"])
    }

    @Test
    fun deleteSurfaceShape() {
        val message = A2uiMessages.deleteSurface("report")
        assertEquals(setOf("version", "deleteSurface"), message.keys)
        assertEquals("report", message["deleteSurface"]!!.jsonObject["surfaceId"]?.jsonPrimitive?.content)
    }

    @Test
    fun envelopesAreTheFourA2uiMessages() =
        assertEquals(setOf("createSurface", "updateComponents", "updateDataModel", "deleteSurface"), A2uiMessages.envelopes)
}
