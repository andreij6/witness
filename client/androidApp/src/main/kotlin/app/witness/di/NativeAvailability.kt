package app.witness.di

import app.witness.BuildConfig
import app.witness.core.data.ImageFiles
import app.witness.core.domain.Clock
import app.witness.core.domain.port.CaseBundleCodec
import app.witness.core.domain.port.IndexGateway
import app.witness.core.domain.port.KeyVault
import app.witness.core.domain.port.MediaGateway
import app.witness.core.domain.port.PreviewPublisher
import app.witness.core.domain.port.RandomSource
import app.witness.core.domain.port.RegistryGateway
import app.witness.core.domain.port.SealEngine
import app.witness.core.domain.port.VerifyEngine
import app.witness.core.icp.IcpEndpoint
import app.witness.core.model.ByteString
import app.witness.core.model.KeyId
import app.witness.feature.proof.CaseLinkOpener
import app.witness.feature.proof.ReportSender
import app.witness.feature.settings.IdentityAccount
import app.witness.nativelib.NativeLibrary
import java.io.File

class NativePorts(
    val seal: SealEngine,
    val verify: VerifyEngine,
    val registry: RegistryGateway,
    val index: IndexGateway,
    val media: MediaGateway,
    val bundles: CaseBundleCodec,
    val identity: IdentityAccount,
    val caseLinks: CaseLinkOpener,
    val reports: ReportSender,
    val linked: Boolean,
    val restore: suspend (KeyVault) -> Unit = {},
    val previews: PreviewPublisher = PreviewPublisher.None,
)

class NativeEnvironment(
    val clock: Clock,
    val images: ImageFiles,
    val vault: KeyVault,
    val files: File,
    val random: RandomSource,
    val certificate: suspend (KeyId) -> ByteString? = { null },
)

fun interface NativePortsFactory {
    fun create(env: NativeEnvironment): NativePorts
}

object NativeAvailability {
    val endpoint: IcpEndpoint? = endpointOf(BuildConfig.ICP_REPLICA_URL, BuildConfig.ICP_REGISTRY_ID, BuildConfig.ICP_FETCH_ROOT_KEY, BuildConfig.WITNESS_CA_ROOT)

    val factory: NativePortsFactory by lazy { select(endpoint, ::loads) }

    fun select(endpoint: IcpEndpoint?, loads: () -> Boolean): NativePortsFactory =
        if (endpoint != null && loads()) RealNative(endpoint) else StandInNative

    fun endpointOf(url: String, registryId: String, fetchRootKey: Boolean, caRoot: String = ""): IcpEndpoint? = when {
        url.isBlank() || registryId.isBlank() -> null
        !isAnchor(caRoot) && (caRoot.isNotEmpty() || !fetchRootKey) -> null
        else -> IcpEndpoint(url, fetchRootKey, registryId, caRoot)
    }

    fun isAnchor(hex: String): Boolean = hex.length >= MIN_ANCHOR_HEX && hex.length % 2 == 0 && hex.startsWith("30") && hex.all { it in '0'..'9' || it in 'a'..'f' }

    private const val MIN_ANCHOR_HEX = 64

    fun loads(): Boolean =
        try {
            System.loadLibrary(NativeLibrary.NAME)
            true
        } catch (_: LinkageError) {
            false
        }
}
