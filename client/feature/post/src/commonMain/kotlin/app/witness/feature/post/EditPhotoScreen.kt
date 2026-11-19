package app.witness.feature.post

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.ColorMatrix
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.witness.core.designsystem.ChipColors
import app.witness.core.designsystem.Witness
import app.witness.core.designsystem.WitnessButton
import app.witness.core.designsystem.WitnessCameraTheme
import app.witness.core.designsystem.WitnessChip
import app.witness.core.designsystem.WitnessColors
import app.witness.core.designsystem.WitnessIcon
import app.witness.core.designsystem.WitnessIconButton
import app.witness.core.designsystem.WIcon
import app.witness.core.designsystem.WitnessShapes
import app.witness.core.designsystem.solidChip
import app.witness.core.model.Edit
import app.witness.core.ui.NormRect

private val EditorDivider = Color(0xFF23262C)
private const val CropDimAlpha = 0.7f
private val HandleSlop = 28.dp

@Composable
fun EditPhotoScreen(editor: EditorState, photo: Photo?, actions: ComposerActions, modifier: Modifier = Modifier) {
    WitnessCameraTheme {
        Column(modifier.fillMaxSize().background(WitnessColors.Camera).statusBarsPadding().navigationBarsPadding()) {
            Box(Modifier.fillMaxWidth().padding(start = 8.dp, end = 16.dp, top = 8.dp).height(40.dp)) {
                WitnessIconButton(WitnessIcon.Close, "Discard edits", actions::closeEditor, Modifier.align(Alignment.CenterStart), tint = WitnessColors.White)
                Text(PostCopy.EDIT_TITLE, Modifier.align(Alignment.Center), style = Witness.type.actionTitle.copy(fontSize = 17.sp), color = WitnessColors.White)
                WitnessButton("Done", actions::applyEditor, Modifier.align(Alignment.CenterEnd).width(84.dp), height = 36.dp, textStyle = Witness.type.bodyStrong)
            }
            Spacer(Modifier.height(22.dp))
            EditCanvas(editor, photo, actions, Modifier.weight(1f).fillMaxWidth().padding(horizontal = 16.dp))
            ToolOptions(editor, actions, Modifier.fillMaxWidth().padding(vertical = 20.dp))
            Box(Modifier.fillMaxWidth().height(1.dp).background(EditorDivider))
            ToolBar(editor.tool) { tool -> actions.editor { it.copy(tool = tool) } }
            Row(Modifier.fillMaxWidth().padding(top = 36.dp, bottom = 24.dp), horizontalArrangement = Arrangement.Center, verticalAlignment = Alignment.CenterVertically) {
                WIcon(WitnessIcon.Edit, WitnessColors.CameraInk2, size = 16.dp)
                Spacer(Modifier.width(6.dp))
                Text(PostCopy.EDIT_NOTE, style = Witness.type.caption, color = WitnessColors.CameraInk2)
            }
        }
    }
}

@Composable
private fun EditCanvas(editor: EditorState, photo: Photo?, actions: ComposerActions, modifier: Modifier) {
    val current by rememberUpdatedState(editor)
    val slop = with(LocalDensity.current) { HandleSlop.toPx() }
    val filter = remember(editor.exposureE2, editor.contrastE2, editor.saturationE2) {
        if (PhotoEditor.isIdentityTone(editor)) null
        else ColorFilter.colorMatrix(ColorMatrix(PhotoEditor.toneMatrix(editor.exposureE2, editor.contrastE2, editor.saturationE2)))
    }
    BoxWithConstraints(modifier.clipToBounds()) {
        val size = with(LocalDensity.current) { Size(maxWidth.toPx(), maxHeight.toPx()) }
        val frame = fitFrame(editor.dims, size)
        val scale = PhotoEditor.coverScale(editor.rotateE2, editor.dims.width, editor.dims.height)
        Box(
            Modifier.fillMaxSize().graphicsLayer {
                rotationZ = editor.rotateE2 / 100f
                scaleX = scale
                scaleY = scale
            },
        ) {
            PhotoImage(
                photo,
                "Photo being edited",
                Modifier.fillMaxSize(),
                contentScale = ContentScale.Fit,
                colorFilter = filter,
                placeholder = WitnessColors.CameraSurface,
                blurs = remember(editor.blurs, editor.dims) { blurEdits(editor.blurs, editor.dims) },
                sourceWidth = editor.dims.width,
            )
        }
        Spacer(
            Modifier
                .fillMaxSize()
                .cropOverlay(editor, frame)
                .pointerInput(editor.tool, frame) {
                    if (current.tool == EditTool.BLUR) {
                        detectTapGestures { p ->
                            val k = frame.width / current.dims.width
                            actions.editor { PhotoEditor.tapBlur(it, ((p.x - frame.left) / k).toInt(), ((p.y - frame.top) / k).toInt()) }
                        }
                    } else if (current.tool == EditTool.CROP) {
                        var handle: CropHandle? = null
                        var accX = 0f
                        var accY = 0f
                        detectDragGestures(
                            onDragStart = { p ->
                                val k = frame.width / current.dims.width
                                handle = PhotoEditor.handleAt(current.crop, (p.x - frame.left) / k, (p.y - frame.top) / k, slop / k)
                                accX = 0f
                                accY = 0f
                            },
                        ) { change, amount ->
                            val h = handle ?: return@detectDragGestures
                            change.consume()
                            val k = frame.width / current.dims.width
                            accX += amount.x / k
                            accY += amount.y / k
                            val dx = accX.toInt()
                            val dy = accY.toInt()
                            if (dx != 0 || dy != 0) {
                                accX -= dx
                                accY -= dy
                                actions.editor { PhotoEditor.drag(it, h, dx, dy) }
                            }
                        }
                    }
                }
                .semantics { contentDescription = "Crop area" },
        )
    }
}

private fun Modifier.cropOverlay(editor: EditorState, frame: Rect): Modifier = drawWithCache {
    val dims = editor.dims
    val c = editor.crop
    val crop = if (dims.isValid) {
        frame.place(NormRect(c.x / dims.width.toFloat(), c.y / dims.height.toFloat(), c.right / dims.width.toFloat(), c.bottom / dims.height.toFloat()))
    } else frame
    val dim = WitnessColors.Black.copy(alpha = CropDimAlpha)
    val thin = Stroke(1.dp.toPx())
    val thick = 4.dp.toPx()
    val arm = 24.dp.toPx()
    val half = 12.dp.toPx()
    val dash = Stroke(1.5.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(6.dp.toPx(), 4.dp.toPx())))
    val showCrop = editor.tool == EditTool.CROP
    val hidden = if (editor.tool == EditTool.BLUR) editor.blurs.filter { !it.enabled }.mapNotNull { b -> b.normalized(dims)?.let(frame::place) } else emptyList()
    onDrawBehind {
        dimOutside(frame, crop, dim)
        if (showCrop) {
            drawRect(WitnessColors.White.copy(alpha = 0.9f), crop.topLeft, crop.size, style = thin)
            val grid = WitnessColors.White.copy(alpha = 0.35f)
            for (k in 1..2) {
                val x = crop.left + crop.width * k / 3f
                val y = crop.top + crop.height * k / 3f
                drawLine(grid, Offset(x, crop.top), Offset(x, crop.bottom), thin.width)
                drawLine(grid, Offset(crop.left, y), Offset(crop.right, y), thin.width)
            }
            cropHandles(crop, arm, half, thick)
        }
        for (r in hidden) drawRect(WitnessColors.White, r.topLeft, r.size, style = dash)
    }
}

private fun DrawScope.dimOutside(frame: Rect, crop: Rect, color: Color) {
    drawRect(color, frame.topLeft, Size(frame.width, crop.top - frame.top))
    drawRect(color, Offset(frame.left, crop.bottom), Size(frame.width, frame.bottom - crop.bottom))
    drawRect(color, Offset(frame.left, crop.top), Size(crop.left - frame.left, crop.height))
    drawRect(color, Offset(crop.right, crop.top), Size(frame.right - crop.right, crop.height))
}

private fun DrawScope.cropHandles(r: Rect, arm: Float, half: Float, width: Float) {
    val white = WitnessColors.White
    fun corner(x: Float, y: Float, sx: Float, sy: Float) {
        drawLine(white, Offset(x, y), Offset(x + arm * sx, y), width, StrokeCap.Square)
        drawLine(white, Offset(x, y), Offset(x, y + arm * sy), width, StrokeCap.Square)
    }
    corner(r.left, r.top, 1f, 1f)
    corner(r.right, r.top, -1f, 1f)
    corner(r.left, r.bottom, 1f, -1f)
    corner(r.right, r.bottom, -1f, -1f)
    val cx = r.center.x
    val cy = r.center.y
    drawLine(white, Offset(cx - half, r.top), Offset(cx + half, r.top), width)
    drawLine(white, Offset(cx - half, r.bottom), Offset(cx + half, r.bottom), width)
    drawLine(white, Offset(r.left, cy - half), Offset(r.left, cy + half), width)
    drawLine(white, Offset(r.right, cy - half), Offset(r.right, cy + half), width)
}

private val UnselectedChip = ChipColors(WitnessColors.White.copy(alpha = 0.12f), WitnessColors.White, WitnessColors.White)

@Composable
private fun ToolOptions(editor: EditorState, actions: ComposerActions, modifier: Modifier) {
    Box(modifier, contentAlignment = Alignment.Center) {
        when (editor.tool) {
            EditTool.CROP -> Row(Modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (a in CropAspect.entries) {
                    val on = a == editor.aspect
                    WitnessChip(
                        a.label,
                        if (on) Witness.colors.solidChip(WitnessColors.White, WitnessColors.Ink) else UnselectedChip,
                        Modifier.selectable(on, role = Role.RadioButton) { actions.editor { PhotoEditor.withAspect(it, a) } },
                        height = 32.dp,
                        fontSize = 12.5.sp,
                    )
                }
            }
            EditTool.STRAIGHTEN -> LabeledSlider("Angle", PhotoEditor.degreesText(editor.rotateE2), editor.rotateE2, -Edit.MAX_ROTATE_DEG_E2..Edit.MAX_ROTATE_DEG_E2) { v ->
                actions.editor { PhotoEditor.straighten(it, v) }
            }
            EditTool.TONE -> Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (ch in ToneChannel.entries) {
                    val v = editor.tone(ch)
                    LabeledSlider(ch.label, PhotoEditor.toneText(v), v, -PhotoEditor.MAX_TONE_E2..PhotoEditor.MAX_TONE_E2) { n -> actions.editor { PhotoEditor.setTone(it, ch, n) } }
                }
            }
            EditTool.BLUR -> Text(
                PostCopy.BLUR_TOOL_NOTE,
                Modifier.padding(horizontal = 32.dp),
                style = Witness.type.caption,
                color = WitnessColors.CameraInk2,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun LabeledSlider(label: String, valueText: String, value: Int, range: IntRange, onChange: (Int) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.width(84.dp), style = Witness.type.label, color = WitnessColors.CameraInk2)
        Slider(
            value = value.toFloat(),
            onValueChange = { onChange(it.toInt()) },
            modifier = Modifier.weight(1f).height(32.dp),
            valueRange = range.first.toFloat()..range.last.toFloat(),
            colors = SliderDefaults.colors(
                thumbColor = WitnessColors.White,
                activeTrackColor = WitnessColors.White,
                inactiveTrackColor = WitnessColors.White.copy(alpha = 0.24f),
            ),
        )
        Text(valueText, Modifier.width(52.dp), style = Witness.type.proof, color = WitnessColors.White, textAlign = TextAlign.End)
    }
}

@Composable
private fun ToolBar(selected: EditTool, onSelect: (EditTool) -> Unit) {
    Row(Modifier.fillMaxWidth().padding(top = 16.dp).selectableGroup()) {
        for (t in EditTool.entries) {
            val on = t == selected
            val color = if (on) WitnessColors.White else WitnessColors.CameraInk2
            Column(
                Modifier.weight(1f).selectable(on, role = Role.Tab) { onSelect(t) },
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Box(
                    Modifier.size(60.dp, 32.dp).clip(WitnessShapes.Pill).background(if (on) WitnessColors.White.copy(alpha = 0.14f) else Color.Transparent),
                    contentAlignment = Alignment.Center,
                ) { WIcon(t.icon, color) }
                Spacer(Modifier.height(6.dp))
                Text(t.label, style = Witness.type.nav.copy(fontWeight = if (on) FontWeight.SemiBold else FontWeight.Medium), color = color)
            }
        }
    }
}
