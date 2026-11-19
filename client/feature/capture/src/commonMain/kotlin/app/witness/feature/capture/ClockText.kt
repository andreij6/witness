package app.witness.feature.capture

import app.witness.core.domain.Clock
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

object ClockText {
    const val DAY_MS: Long = 86_400_000L

    fun format(epochMs: Long, offsetMs: Long = 0L): String {
        val local = (epochMs + offsetMs).mod(DAY_MS)
        val seconds = (local / 1000).toInt()
        val h24 = seconds / 3600
        val h12 = if (h24 % 12 == 0) 12 else h24 % 12
        val out = CharArray(if (h12 >= 10) 11 else 10)
        var p = 0
        if (h12 >= 10) out[p++] = '1'
        out[p++] = '0' + h12 % 10
        p = twoDigits(out, p, ':', seconds / 60 % 60)
        p = twoDigits(out, p, ':', seconds % 60)
        out[p++] = ' '
        out[p++] = if (h24 < 12) 'A' else 'P'
        out[p] = 'M'
        return out.concatToString()
    }

    fun everySecond(clock: Clock): Flow<Long> = flow {
        while (true) {
            val now = clock.nowEpochMillis()
            emit(now)
            delay(1000 - now.mod(1000L))
        }
    }

    private fun twoDigits(out: CharArray, start: Int, separator: Char, value: Int): Int {
        out[start] = separator
        out[start + 1] = '0' + value / 10
        out[start + 2] = '0' + value % 10
        return start + 3
    }
}
