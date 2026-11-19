package app.witness.di

import app.witness.BuildConfig
import android.util.Log
import app.witness.core.domain.port.EnrollOutcome
import app.witness.core.domain.port.EnrollmentRequest
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.usecase.CaseBundleCbor
import app.witness.core.icp.IcpEndpoint
import app.witness.core.model.RecordId
import app.witness.feature.proof.CaseLinkResult
import app.witness.feature.proof.InMemoryReportSender
import app.witness.feature.settings.fake.InMemoryIdentityAccount
import app.witness.nativelib.ClaimFiles
import app.witness.nativelib.Hex
import app.witness.nativelib.NativeIndex
import app.witness.nativelib.NativeKeys
import app.witness.nativelib.NativeMedia
import app.witness.nativelib.NativePrepare
import app.witness.nativelib.NativeRegistry
import app.witness.nativelib.NativeSealEngine
import app.witness.nativelib.NativeTrust
import app.witness.nativelib.NativeVerifyEngine
import uniffi.witness_android.CaseInfo
import uniffi.witness_android.Witness
import uniffi.witness_android.WitnessConfig
import uniffi.witness_android.WitnessInterface
import uniffi.witness_android.caseInfo
import uniffi.witness_android.decodeClaim
import uniffi.witness_android.prepare
import java.io.File

class RealNative(private val endpoint: IcpEndpoint) : NativePortsFactory {
    override fun create(env: NativeEnvironment): NativePorts {
        val witness = Witness(config(endpoint, identitySeed(File(env.files, DIR), env.random)))
        return ports(witness, env) { shard, id -> caseInfo(witness, shard, id) }
    }

    fun ports(witness: WitnessInterface, env: NativeEnvironment, caseInfo: suspend (UByte, String) -> CaseInfo? = { _, _ -> null }): NativePorts {
        val dir = File(env.files, DIR)
        val trust = NativeTrust(witness, File(dir, CA_FILE), pinnedAnchor(endpoint))
        val keys = NativeKeys(env.vault, trust, env.certificate)
        val registry = NativeRegistry(witness, keys, trust, ::decodeClaim)
        val media = NativeMedia(witness, keys, endpoint.host, previewOf = PreviewJpeg::encode, caseInfo = caseInfo)
        return NativePorts(
            seal = NativeSealEngine(
                keys = keys,
                images = env.images::read,
                claims = ClaimFiles(File(dir, CLAIMS_DIR)),
                remoteClaim = registry::claimCbor,
                renderer = BitmapEditRenderer,
                random = env.random,
                appVersion = BuildConfig.VERSION_NAME,
                prepareNative = NativePrepare { jpeg, meta -> prepare(jpeg, meta, null) },
                decode = ::decodeClaim,
            ),
            verify = NativeVerifyEngine(witness),
            registry = object : RegistryGateway by registry {
                override suspend fun enroll(request: EnrollmentRequest) =
                    registry.enroll(request).also { if (it is EnrollOutcome.Rejected) Log.w(TAG, "enrollment rejected: ${it.reason}") }
            },
            index = NativeIndex(witness),
            media = object : MediaGateway by media {
                override suspend fun reference(recordId: RecordId) = env.images.read(recordId) ?: media.reference(recordId)
            },
            bundles = CaseBundleCbor,
            identity = InMemoryIdentityAccount(),
            caseLinks = { _, _ -> CaseLinkResult.NotFound },
            reports = InMemoryReportSender(),
            linked = true,
            previews = media,
        )
    }

    companion object {
        fun pinnedAnchor(endpoint: IcpEndpoint): ByteArray? = endpoint.caRoot.takeIf { it.isNotEmpty() }?.let(Hex::parse)

        fun config(endpoint: IcpEndpoint, identitySeed: ByteArray? = null): WitnessConfig =
            WitnessConfig(endpoint.host, endpoint.registryId, endpoint.fetchRootKey, pinnedAnchor(endpoint), identitySeed)

        fun identitySeed(dir: File, random: RandomSource): ByteArray {
            val file = File(dir, SEED_FILE)
            if (file.isFile) file.readBytes().takeIf { it.size == SEED_BYTES }?.let { return it }
            dir.mkdirs()
            val seed = random.nextBytes(SEED_BYTES)
            val staged = File(dir, "$SEED_FILE.tmp")
            staged.writeBytes(seed)
            check(staged.renameTo(file)) { "cannot store the network identity" }
            return seed
        }

        const val TAG = "Witness"
        const val DIR = "native"
        const val CA_FILE = "witness-ca.der"
        const val SEED_FILE = "identity.seed"
        const val SEED_BYTES = 32
        const val CLAIMS_DIR = "claims"
    }
}
