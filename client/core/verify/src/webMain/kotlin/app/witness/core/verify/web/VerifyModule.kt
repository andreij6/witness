@file:JsModule("witness-bridges/verify.mjs")

package app.witness.core.verify.web

import org.khronos.webgl.Uint8Array
import kotlin.js.JsAny
import kotlin.js.JsBoolean
import kotlin.js.Promise

external interface JudgementJs : JsAny {
    val verdict: Int
    val recordId: String?
    val flags: Int
    val fallback: Boolean
}

external fun judge(url: String?, bytes: Uint8Array, onProgress: (Int, Int, Int, Int) -> Unit): Promise<JudgementJs>

external fun isFallback(url: String?): Promise<JsBoolean>
