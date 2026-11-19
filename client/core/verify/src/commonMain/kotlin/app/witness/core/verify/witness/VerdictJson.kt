package app.witness.core.verify.witness

import app.witness.core.icp.witness.WireException
import app.witness.core.icp.witness.WireJson
import app.witness.core.icp.witness.WireJson.array
import app.witness.core.icp.witness.WireJson.asObject
import app.witness.core.icp.witness.WireJson.int
import app.witness.core.icp.witness.WireJson.intOrNull
import app.witness.core.icp.witness.WireJson.obj
import app.witness.core.icp.witness.WireJson.objOrNull
import app.witness.core.icp.witness.WireJson.recordId
import app.witness.core.icp.witness.WireJson.str
import app.witness.core.icp.witness.WireJson.u64
import app.witness.core.model.Anchor
import app.witness.core.model.CheckResult
import app.witness.core.model.MatchEvidence
import app.witness.core.model.Reference
import app.witness.core.model.Region
import app.witness.core.model.Verdict
import app.witness.core.model.VerdictFlag
import app.witness.core.model.VerdictReason
import app.witness.core.model.enumBitsOrNull
import kotlinx.serialization.json.JsonObject

object VerdictJson {
    fun parse(text: String): CheckResult = result(WireJson.parse(text))

    fun result(o: JsonObject): CheckResult = CheckResult(
        verdict = WireJson.name(o.str("outcome"), Verdict.entries),
        flags = enumBitsOrNull<VerdictFlag>(o.int("flags")) ?: throw WireException("flags"),
        reasons = enumBitsOrNull<VerdictReason>(o.intOrNull("reasonBits") ?: 0) ?: throw WireException("reasonBits"),
        declared = o.array("declared").map { WireJson.edit(it.asObject()) },
        regions = o.array("regions").map { r -> r.asObject().let { Region(it.int("x"), it.int("y"), it.int("w"), it.int("h")) } },
        reference = o.objOrNull("reference")?.let(::reference),
        version = o.intOrNull("version") ?: CheckResult.VERDICT_VERSION,
    )

    fun reference(o: JsonObject): Reference = Reference(
        recordId = o.recordId("recordId"),
        claim = WireJson.claim(o.obj("claim")),
        anchor = o.objOrNull("anchor")?.let { Anchor(it.u64("recordIx"), it.u64("anchoredNs") / WireJson.NS_PER_MS) },
        evidence = WireJson.name(o.str("evidence"), MatchEvidence.entries),
        confidenceQ16 = o.int("confidenceQ16"),
        comparedQ16 = o.int("comparedQ16"),
        changedQ16 = o.int("changedQ16"),
    )
}
