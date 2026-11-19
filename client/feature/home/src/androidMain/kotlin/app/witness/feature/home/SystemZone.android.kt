package app.witness.feature.home

import java.util.TimeZone

actual fun systemZoneOffset(epochMs: Long): Int = TimeZone.getDefault().getOffset(epochMs)
