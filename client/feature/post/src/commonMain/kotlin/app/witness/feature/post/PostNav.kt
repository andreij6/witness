package app.witness.feature.post

import androidx.navigation3.runtime.NavKey
import app.witness.core.model.LocationPrecision
import app.witness.core.model.RecordId
import kotlinx.serialization.Serializable

@Serializable
data class PostComposerKey(val parent: String) : NavKey {
    val recordId: RecordId get() = RecordId.parse(parent)

    companion object {
        fun of(parent: RecordId): PostComposerKey = PostComposerKey(parent.text)
    }
}

@Serializable
data class PostSharedKey(val record: String, val precision: LocationPrecision) : NavKey {
    val recordId: RecordId get() = RecordId.parse(record)

    companion object {
        fun of(recordId: RecordId, precision: LocationPrecision): PostSharedKey = PostSharedKey(recordId.text, precision)
    }
}
