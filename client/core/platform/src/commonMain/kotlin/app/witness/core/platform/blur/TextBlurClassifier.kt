package app.witness.core.platform.blur

import app.witness.core.model.BlurClass

object TextBlurClassifier {
    private const val PLATE_MIN = 5
    private const val PLATE_MAX = 8
    private const val HOUSE_MAX_DIGITS = 5

    fun classify(text: String): BlurClass? {
        val t = text.trim()
        if (t.isEmpty() || t.length > PLATE_MAX + 2) return null
        if (isHouseNumber(t)) return BlurClass.HOUSE_NO
        if (isPlate(t)) return BlurClass.PLATE
        return null
    }

    fun isHouseNumber(t: String): Boolean {
        var digits = 0
        var i = 0
        while (i < t.length && t[i] in '0'..'9') { digits++; i++ }
        if (digits == 0 || digits > HOUSE_MAX_DIGITS || t[0] == '0') return false
        if (i == t.length) return true
        return i == t.length - 1 && t[i].isLetter()
    }

    fun isPlate(t: String): Boolean {
        var letters = 0
        var digits = 0
        var separators = 0
        for (c in t) {
            when {
                c in '0'..'9' -> digits++
                c in 'A'..'Z' -> letters++
                c == ' ' || c == '-' || c == '·' -> separators++
                else -> return false
            }
        }
        val len = letters + digits
        return letters > 0 && digits > 0 && separators <= 2 && len in PLATE_MIN..PLATE_MAX
    }
}
