package app.witness.core.a2ui

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

object A2uiMessages {
    const val VERSION = "v0.9.1"
    const val SLOTS_PATH = "/slots"

    const val CREATE_SURFACE = "createSurface"
    const val UPDATE_COMPONENTS = "updateComponents"
    const val UPDATE_DATA_MODEL = "updateDataModel"
    const val DELETE_SURFACE = "deleteSurface"

    val envelopes: Set<String> = hashSetOf(CREATE_SURFACE, UPDATE_COMPONENTS, UPDATE_DATA_MODEL, DELETE_SURFACE)

    fun slotPath(slotId: String): String = "$SLOTS_PATH/$slotId"

    fun updateDataModel(surfaceId: String, path: String, value: JsonElement): JsonObject =
        JsonObject(
            mapOf(
                "version" to JsonPrimitive(VERSION),
                UPDATE_DATA_MODEL to JsonObject(
                    mapOf(
                        "surfaceId" to JsonPrimitive(surfaceId),
                        "path" to JsonPrimitive(path),
                        "value" to value,
                    ),
                ),
            ),
        )

    fun deleteSurface(surfaceId: String): JsonObject =
        JsonObject(
            mapOf(
                "version" to JsonPrimitive(VERSION),
                DELETE_SURFACE to JsonObject(mapOf("surfaceId" to JsonPrimitive(surfaceId))),
            ),
        )
}
