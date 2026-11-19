package app.witness.core.verify.witness

object Verdicts {
    const val RECORD_ID: String = "WERC8GMRZGE196QV"
    const val ANCHOR: String = """{"anchoredNs":"1700000001000000000","recordIx":"9"}"""

    val NO_RECORD: String = """
        {"declared":[],"flagNames":[],"flags":0,"outcome":"NO_RECORD","outcomeCode":3,"reasonBits":4608,
         "reasons":["WatermarkUnavailable","NoCandidates"],"recordId":null,"reference":null,"regions":[],"version":1}
    """.trimIndent()

    val WITH_CHANGES: String = """
        {"declared":[{"class":"face","h":40,"kind":"blur","sigmaE2":800,"w":30,"x":10,"y":20}],
         "flagNames":["COULDNT_COMPARE","DEV_ATTESTATION"],"flags":40,"outcome":"MATCHES_WITH_CHANGES","outcomeCode":1,
         "reasonBits":128,"reasons":["ExactPixelHash"],"recordId":"$RECORD_ID",
         "reference":{"recordId":"$RECORD_ID","claim":{"assetHash":"${"aa".repeat(32)}","captureId":"5","coarse":null,"counter":"1",
           "height":480,"keyId":"${"11".repeat(16)}","kind":{"type":"original"},"pixelHash":"${"bb".repeat(32)}","wallMs":"1700000000000","width":640},
           "anchor":$ANCHOR,"evidence":"exact_hash","confidenceQ16":65536,"comparedQ16":65536,"changedQ16":100},
         "regions":[{"h":4,"w":3,"x":1,"y":2}],"version":1}
    """.trimIndent()
}
