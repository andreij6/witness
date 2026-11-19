@file:JsModule("witness-bridges/image.mjs")

package app.witness.core.verify.web

import org.khronos.webgl.Uint8Array
import kotlin.js.JsAny
import kotlin.js.Promise

external interface DimensionsJs : JsAny {
    val width: Int
    val height: Int
}

external fun dimensions(bytes: Uint8Array): Promise<DimensionsJs?>
