@file:JsModule("witness-bridges/witness.mjs")

package app.witness.core.icp.witness

import org.khronos.webgl.Uint8Array
import kotlin.js.JsAny
import kotlin.js.JsString
import kotlin.js.Promise

external interface CaseJs : JsAny {
    val status: String
    val json: String?
    val bundle: Uint8Array?
}

external fun check(bytes: Uint8Array, onStage: (Int, Int, Int, Int) -> Unit): Promise<JsString>

external fun proof(recordId: String): Promise<JsString?>

external fun key(keyIdHex: String): Promise<JsString?>

external fun preview(recordId: String): Promise<Uint8Array?>

external fun openCase(caseId: String, fragment: String): Promise<CaseJs>
