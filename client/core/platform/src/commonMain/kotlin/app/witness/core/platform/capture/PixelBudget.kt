package app.witness.core.platform.capture

object PixelBudget {
    fun <S> select(sizes: List<S>, maxPixels: Int, width: (S) -> Int, height: (S) -> Int): List<S> {
        val within = ArrayList<S>(sizes.size)
        for (s in sizes) if (width(s).toLong() * height(s) <= maxPixels) within += s
        within.sortByDescending { width(it).toLong() * height(it) }
        return within
    }

    fun rotationIsValid(degrees: Int): Boolean = degrees == 0 || degrees == 90 || degrees == 180 || degrees == 270
}
