package app.witness.core.domain.usecase

import app.witness.core.domain.port.CaseBundle
import app.witness.core.domain.port.CaseBundleItem
import app.witness.core.model.ByteString
import app.witness.core.model.CaseId
import app.witness.core.model.KeyId
import app.witness.core.model.KeyRef
import app.witness.core.model.RecordId
import app.witness.core.model.Submission

object CaseVector {
    val KEY: ByteArray = ByteArray(32) { 0x42 }
    val NONCE: ByteArray = ByteArray(12) { 7 }
    val CASE_RANDOM: ByteArray = ByteArray(15) { 0xab.toByte() }
    val CASE_ID: CaseId = CaseId.of(0, CASE_RANDOM)
    const val CASE_ID_TEXT = "02NTQAXBNENTQAXBNENTQAXBNC"
    const val RECORD_B = "VMDKRC9CYZC1C4R3"
    const val FRAGMENT = "#k=QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI"
    const val HASH = "e94360c3290ec996e280f1ed75ea40525ffa37232abebb9e0b93da9dd5ab3f00"

    val BUNDLE: String = "a40001016b526561722062756d706572026a74776f2070686f746f730382a2004a010101010101010101010145ffd8010203a3004add1b3c312cf7d81613030159012c00" +
        "070e151c232a31383f464d545b626970777e858c939aa1a8afb6bdc4cbd2d9e0e7eef501080f161d242b323940474e555c636a71787f868d949ba2a9b0b7bec5ccd3dae1e8eff6" +
        "020910171e252c333a41484f565d646b727980878e959ca3aab1b8bfc6cdd4dbe2e9f0f7030a11181f262d343b424950575e656c737a81888f969da4abb2b9c0c7ced5dce3eaf1f8" +
        "040b121920272e353c434a51585f666d747b828990979ea5acb3bac1c8cfd6dde4ebf2f9050c131a21282f363d444b525960676e757c838a91989fa6adb4bbc2c9d0d7dee5ecf3fa" +
        "060d141b222930373e454c535a61686f767d848b9299a0a7aeb5bcc3cad1d8dfe6edf400070e151c232a31383f464d545b626970777e858c939aa1a8afb6bdc4cbd2d9e0e7eef501" +
        "080f161d242b323940474e5502a400010145636c61696d0244633270610344636f7365"

    val CHUNK: String = "070707070707070707070707dce25caddea3179a51748e45b02b430ef7307b4a92f5382d0f00502019afba7465e91bd175b2744c46f744899f60b854846c1c296604e67db1" +
        "29a2f6040b26c9399481debfd79b3b9aff4c6ca54a1b417c5f59232306dc763cedbe6e3fd646923c634e3096f61f8f56bf81cc31d9930ff9b046b218fcb1160473f71d7b2a0ae887" +
        "4c76d319732c755719c1d07fedf0a19f43a5a9bb9ff5301caa3e22dd6f3d7f37e55b27a2efcc8406ef62e1065f08fad2cdc38ffe18cf46effe794e879fbcb5f8a01b2a14c330d624" +
        "e78aa7c4344a5ba027829053eaf8c7f43fbf21a79ae329ca8a7e2da81344aec9a41274b98063f602981a0779b80f228283917619da7f26ce2274bfd5c78723d727a70620bc260d4f" +
        "6864af266057c0390b9482f72445e1a499c7c087618230e0c14d14b15ccf956c4689ac99155246c4f27fafb0be127ab294ec593244ca1497b343237a298caed6e5ada562e6c9596b" +
        "a4643283c1aa24433fe99e1590ce9816c7903a14ce87f8f4131159c3b74ee028221d685a75148871a355e9e52a3191a9e012ac122f016f57ab683be3ea"

    val JPEG_A: ByteString = ByteString.of(byteArrayOf(-1, -40, 1, 2, 3))
    val JPEG_B: ByteString = ByteString.of(ByteArray(300) { i -> (i * 7 % 251).toByte() })

    val SUBMISSION: Submission = Submission(
        RecordId.parse(RECORD_B),
        KeyRef(KeyId.fromBytes(ByteArray(16)), 0),
        true,
        0,
        ByteString.of("claim".encodeToByteArray()),
        ByteString.of("c2pa".encodeToByteArray()),
        ByteString.of("cose".encodeToByteArray()),
    )

    val BUNDLE_VALUE: CaseBundle = CaseBundle(
        "Rear bumper",
        "two photos",
        listOf(
            CaseBundleItem(RecordId.fromBytes(ByteArray(10) { 1 }), JPEG_A, null),
            CaseBundleItem(SUBMISSION.recordId, JPEG_B, SUBMISSION),
        ),
    )
}
