package app.witness.core.icp.witness

object WitnessFixtures {
    const val RECORD_ID: String = "WERC8GMRZGE196QV"
    const val PARENT_ID: String = "0123456789ABCDEF"
    const val KEY_HEX: String = "00112233445566778899aabbccddeeff"
    val HASH_A: String = "aa".repeat(32)
    val HASH_B: String = "bb".repeat(32)

    val LEAN_CLAIM: String = """
        {"assetHash":"$HASH_A","captureId":"123456789","coarse":null,"counter":"3","height":480,
         "keyId":"$KEY_HEX","kind":{"type":"original"},"pixelHash":"$HASH_B","wallMs":"1700000000000","width":640}
    """.trimIndent()

    val FULL_CLAIM: String = """
        {"assetHash":"$HASH_A","bootCount":4,"captureId":"123456789","certSerial":"0a0b",
         "checks":{"boot":1,"deviceLocked":true,"gnss":5,"keySecurity":2,"mockLocation":false,"recapture":3},
         "coarse":"9q8yy","counter":"3","elapsedNs":"55","fpCommit":"${"cc".repeat(32)}","gnssMs":"1700000000500","height":480,
         "keyId":"$KEY_HEX","kind":{"type":"derivative","parent":"$PARENT_ID","root":"77","edits":[
           {"kind":"blur","x":1,"y":2,"w":3,"h":4,"class":"house_no","sigmaE2":900},
           {"kind":"recompress","format":"jpeg","quality":80},
           {"kind":"resize","w":10,"h":20,"filter":1},
           {"kind":"orient","orientation":6},
           {"kind":"crop","x":0,"y":0,"w":5,"h":6},
           {"kind":"rotate","degE2":-150},
           {"kind":"tone","exposureE2":-3,"contrastE2":4,"saturationE2":5},
           {"kind":"filter","id":2,"strengthE2":50}]},
         "location":{"commit":"${"dd".repeat(32)}","confidence":1,"reasons":3},
         "pixelHash":"$HASH_B","wallMs":"1700000000000","width":640}
    """.trimIndent()

    val PROOF: String = """{"recordIx":"7","recordId":"$RECORD_ID","anchoredNs":"1700000001000000000","claim":$FULL_CLAIM}"""

    val KEY: String = """
        {"keyId":"$KEY_HEX","pubkey":"04","security":"StrongBox","provisioning":"Rkp","freshness":"Late","patch":202501,
         "enrolledNs":"1000000000","activeUntilNs":"9000000000",
         "history":[{"status":"Active","sinceNs":"1000000000"},{"status":"Revoked","sinceNs":"5000000000"}],"chainSerials":["01"]}
    """.trimIndent()
}

class FakeBridge(
    var checkJson: String = "{}",
    var stages: List<IntArray> = emptyList(),
    var proofs: Map<String, String> = emptyMap(),
    var keys: Map<String, String> = emptyMap(),
    var previews: Map<String, ByteArray> = emptyMap(),
    var opened: OpenedBundle = OpenedBundle(OpenedBundle.NOT_FOUND, null, null),
    var failure: Exception? = null,
) : WitnessBridge {
    val calls = mutableListOf<String>()

    private fun fail() = failure?.let { throw it }

    override suspend fun check(input: ByteArray, listener: StageListener): String {
        calls += "check:${input.size}"
        fail()
        stages.forEach { listener.onStage(it[0], it[1], it[2], it[3]) }
        return checkJson
    }

    override suspend fun proof(recordId: String): String? = proofs[recordId].also { calls += "proof:$recordId"; fail() }

    override suspend fun key(keyIdHex: String): String? = keys[keyIdHex].also { calls += "key:$keyIdHex"; fail() }

    override suspend fun preview(recordId: String): ByteArray? = previews[recordId].also { calls += "preview:$recordId"; fail() }

    override suspend fun openCase(caseId: String, fragment: String): OpenedBundle = opened.also { calls += "case:$caseId"; fail() }
}
