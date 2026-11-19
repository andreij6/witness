package app.witness.system

import app.witness.feature.check.PickedImage

class SharedImages(private val capacity: Int = DEFAULT_CAPACITY) {
    private val items = LinkedHashMap<String, PickedImage>()
    private var next = 0L

    init {
        require(capacity > 0) { "capacity must be positive" }
    }

    @Synchronized
    fun put(image: PickedImage): String {
        val id = (next++).toString(RADIX)
        items[id] = image
        if (items.size > capacity) items.remove(items.keys.first())
        return id
    }

    fun stash(read: () -> PickedImage?): String {
        val image = try {
            read()
        } catch (_: SecurityException) {
            null
        } catch (_: java.io.IOException) {
            null
        } catch (_: IllegalArgumentException) {
            null
        }
        return image?.let(::put) ?: MISSING
    }

    @Synchronized
    fun get(id: String): PickedImage? = items[id]

    @get:Synchronized
    val size: Int get() = items.size

    companion object {
        const val DEFAULT_CAPACITY: Int = 2
        const val MISSING: String = "missing"
        private const val RADIX = 36
    }
}
