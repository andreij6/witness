package app.witness.core.designsystem

import androidx.compose.runtime.Composable
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import app.witness.core.designsystem.generated.Res
import app.witness.core.designsystem.generated.inter
import app.witness.core.designsystem.generated.jetbrains_mono
import org.jetbrains.compose.resources.Font

internal val SansWeights = listOf(FontWeight.Normal, FontWeight.Medium, FontWeight.SemiBold, FontWeight.Bold)
internal val MonoWeights = listOf(FontWeight.Normal, FontWeight.Medium, FontWeight.Bold)

@Composable
fun interFontFamily(): FontFamily = FontFamily(SansWeights.map { Font(Res.font.inter, it) })

@Composable
fun jetBrainsMonoFontFamily(): FontFamily = FontFamily(MonoWeights.map { Font(Res.font.jetbrains_mono, it) })
