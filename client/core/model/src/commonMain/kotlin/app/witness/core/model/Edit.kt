package app.witness.core.model

enum class EditClass { PRESENTATION, GLOBAL, LOCAL_DECLARED }

enum class EditKind(val editClass: EditClass) {
    RESIZE(EditClass.PRESENTATION),
    RECOMPRESS(EditClass.PRESENTATION),
    ORIENT(EditClass.PRESENTATION),
    CROP(EditClass.GLOBAL),
    ROTATE(EditClass.GLOBAL),
    TONE(EditClass.GLOBAL),
    FILTER(EditClass.GLOBAL),
    BLUR(EditClass.LOCAL_DECLARED),
}

enum class BlurClass { FACE, PLATE, HOUSE_NO, OTHER }

enum class ImageFormat { JPEG, PNG, WEBP, AVIF, HEIC }

data class Rect(val x: Int, val y: Int, val w: Int, val h: Int) {
    val isValid: Boolean
        get() = x >= 0 && y >= 0 && w > 0 && h > 0 && x.toLong() + w <= UINT_MAX && y.toLong() + h <= UINT_MAX

    val area: Long get() = w.toLong() * h.toLong()
    val right: Int get() = x + w
    val bottom: Int get() = y + h

    fun clippedTo(width: Int, height: Int): Rect? {
        val l = x.coerceIn(0, width)
        val t = y.coerceIn(0, height)
        val r = (x.toLong() + w).coerceIn(0, width.toLong()).toInt()
        val b = (y.toLong() + h).coerceIn(0, height.toLong()).toInt()
        return if (r > l && b > t) Rect(l, t, r - l, b - t) else null
    }

    private companion object {
        const val UINT_MAX = 0xffff_ffffL
    }
}

sealed interface Edit {
    val kind: EditKind
    val isValid: Boolean
    val editClass: EditClass get() = kind.editClass

    data class Resize(val w: Int, val h: Int, val filter: Int = 0) : Edit {
        override val kind get() = EditKind.RESIZE
        override val isValid get() = w > 0 && h > 0 && filter in 0..255
    }

    data class Recompress(val format: ImageFormat, val quality: Int) : Edit {
        override val kind get() = EditKind.RECOMPRESS
        override val isValid get() = quality in 0..100
    }

    data class Orient(val orientation: Int) : Edit {
        override val kind get() = EditKind.ORIENT
        override val isValid get() = orientation in 0..MAX_ORIENT
    }

    data class Crop(val rect: Rect) : Edit {
        override val kind get() = EditKind.CROP
        override val isValid get() = rect.isValid
    }

    data class Rotate(val degE2: Int) : Edit {
        override val kind get() = EditKind.ROTATE
        override val isValid get() = degE2 in -MAX_ROTATE_DEG_E2..MAX_ROTATE_DEG_E2
    }

    data class Tone(val exposureE2: Int, val contrastE2: Int, val saturationE2: Int) : Edit {
        override val kind get() = EditKind.TONE
        override val isValid get() = exposureE2 in I16 && contrastE2 in I16 && saturationE2 in I16
    }

    data class Filter(val id: Int, val strengthE2: Int) : Edit {
        override val kind get() = EditKind.FILTER
        override val isValid get() = id in U16 && strengthE2 in U16
    }

    data class Blur(val rect: Rect, val blurClass: BlurClass, val sigmaE2: Int) : Edit {
        override val kind get() = EditKind.BLUR
        override val isValid get() = rect.isValid && sigmaE2 in U16
    }

    companion object {
        const val MAX_ROTATE_DEG_E2: Int = 1000
        const val MAX_ORIENT: Int = 7
        const val MAX_EDITS: Int = 64
        private val I16 = Short.MIN_VALUE..Short.MAX_VALUE
        private val U16 = 0..0xffff
    }
}

fun List<Edit>.isPresentationOnly(): Boolean = all { it.editClass == EditClass.PRESENTATION }

fun List<Edit>.hasLocal(): Boolean = any { it.editClass == EditClass.LOCAL_DECLARED }
