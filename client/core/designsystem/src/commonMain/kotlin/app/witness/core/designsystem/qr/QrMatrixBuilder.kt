package app.witness.core.designsystem.qr

import kotlin.math.abs
import kotlin.math.max

internal class QrMatrixBuilder(private val version: Int, private val ecc: QrEcc) {
    private val size = QrTables.size(version)
    private val modules = BooleanArray(size * size)
    private val function = BooleanArray(size * size)
    private val history = IntArray(7)

    fun build(codewords: ByteArray, forcedMask: Int, mode: QrMode): QrMatrix {
        drawFunctionPatterns()
        drawCodewords(codewords)
        val mask = if (forcedMask >= 0) forcedMask else bestMask()
        applyMask(mask)
        drawFormatBits(mask)
        return QrMatrix(version, ecc, mask, mode, modules)
    }

    private fun bestMask(): Int {
        var best = 0
        var bestPenalty = Int.MAX_VALUE
        for (m in 0 until 8) {
            applyMask(m)
            drawFormatBits(m)
            val p = penalty()
            if (p < bestPenalty) {
                best = m
                bestPenalty = p
            }
            applyMask(m)
        }
        return best
    }

    private fun set(x: Int, y: Int, dark: Boolean) {
        val i = y * size + x
        modules[i] = dark
        function[i] = true
    }

    private fun drawFunctionPatterns() {
        for (i in 0 until size) {
            set(6, i, i % 2 == 0)
            set(i, 6, i % 2 == 0)
        }
        drawFinder(3, 3)
        drawFinder(size - 4, 3)
        drawFinder(3, size - 4)
        val align = QrTables.alignmentPositions(version)
        val n = align.size
        for (i in 0 until n) for (j in 0 until n) {
            if (i == 0 && j == 0 || i == 0 && j == n - 1 || i == n - 1 && j == 0) continue
            drawAlignment(align[i], align[j])
        }
        drawFormatBits(0)
        drawVersion()
    }

    private fun drawFinder(cx: Int, cy: Int) {
        for (dy in -4..4) for (dx in -4..4) {
            val x = cx + dx
            val y = cy + dy
            if (x !in 0 until size || y !in 0 until size) continue
            val d = max(abs(dx), abs(dy))
            set(x, y, d != 2 && d != 4)
        }
    }

    private fun drawAlignment(cx: Int, cy: Int) {
        for (dy in -2..2) for (dx in -2..2) set(cx + dx, cy + dy, max(abs(dx), abs(dy)) != 1)
    }

    private fun drawFormatBits(mask: Int) {
        val bits = QrTables.formatBits(ecc, mask)
        fun bit(i: Int) = bits ushr i and 1 != 0
        for (i in 0..5) set(8, i, bit(i))
        set(8, 7, bit(6))
        set(8, 8, bit(7))
        set(7, 8, bit(8))
        for (i in 9 until 15) set(14 - i, 8, bit(i))
        for (i in 0 until 8) set(size - 1 - i, 8, bit(i))
        for (i in 8 until 15) set(8, size - 15 + i, bit(i))
        set(8, size - 8, true)
    }

    private fun drawVersion() {
        if (version < 7) return
        val bits = QrTables.versionBits(version)
        for (i in 0 until 18) {
            val dark = bits ushr i and 1 != 0
            val a = size - 11 + i % 3
            val b = i / 3
            set(a, b, dark)
            set(b, a, dark)
        }
    }

    private fun drawCodewords(data: ByteArray) {
        val totalBits = data.size * 8
        var i = 0
        var right = size - 1
        while (right >= 1) {
            if (right == 6) right = 5
            val upward = (right + 1) and 2 == 0
            for (vert in 0 until size) {
                val y = if (upward) size - 1 - vert else vert
                for (j in 0..1) {
                    val idx = y * size + right - j
                    if (!function[idx] && i < totalBits) {
                        modules[idx] = (data[i ushr 3].toInt() ushr (7 - (i and 7))) and 1 != 0
                        i++
                    }
                }
            }
            right -= 2
        }
    }

    private fun applyMask(mask: Int) {
        for (y in 0 until size) for (x in 0 until size) {
            val idx = y * size + x
            if (!function[idx] && masked(mask, x, y)) modules[idx] = !modules[idx]
        }
    }

    private fun dark(x: Int, y: Int) = modules[y * size + x]

    private fun penalty(): Int {
        var result = 0
        for (y in 0 until size) result += linePenalty { dark(it, y) }
        for (x in 0 until size) result += linePenalty { dark(x, it) }
        for (y in 0 until size - 1) for (x in 0 until size - 1) {
            val c = dark(x, y)
            if (c == dark(x + 1, y) && c == dark(x, y + 1) && c == dark(x + 1, y + 1)) result += 3
        }
        val darkCount = modules.count { it }
        val total = size * size
        val k = (abs(darkCount * 20 - total * 10) + total - 1) / total - 1
        return result + k * 10
    }

    private inline fun linePenalty(module: (Int) -> Boolean): Int {
        var result = 0
        var runColor = false
        var run = 0
        history.fill(0)
        for (i in 0 until size) {
            if (module(i) == runColor) {
                run++
                if (run == 5) result += 3 else if (run > 5) result++
            } else {
                pushHistory(run)
                if (!runColor) result += finderPatterns() * 40
                runColor = module(i)
                run = 1
            }
        }
        if (runColor) {
            pushHistory(run)
            run = 0
        }
        pushHistory(run + size)
        return result + finderPatterns() * 40
    }

    private fun pushHistory(run: Int) {
        val length = if (history[0] == 0) run + size else run
        for (i in 6 downTo 1) history[i] = history[i - 1]
        history[0] = length
    }

    private fun finderPatterns(): Int {
        val n = history[1]
        val core = n > 0 && history[2] == n && history[3] == n * 3 && history[4] == n && history[5] == n
        return (if (core && history[0] >= n * 4 && history[6] >= n) 1 else 0) +
            (if (core && history[6] >= n * 4 && history[0] >= n) 1 else 0)
    }

    companion object {
        fun masked(mask: Int, x: Int, y: Int): Boolean = when (mask) {
            0 -> (x + y) % 2 == 0
            1 -> y % 2 == 0
            2 -> x % 3 == 0
            3 -> (x + y) % 3 == 0
            4 -> (x / 3 + y / 2) % 2 == 0
            5 -> x * y % 2 + x * y % 3 == 0
            6 -> (x * y % 2 + x * y % 3) % 2 == 0
            7 -> ((x + y) % 2 + x * y % 3) % 2 == 0
            else -> throw IllegalArgumentException("mask out of range: $mask")
        }
    }
}
