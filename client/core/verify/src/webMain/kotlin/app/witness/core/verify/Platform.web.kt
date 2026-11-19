package app.witness.core.verify

import app.witness.core.icp.web.toUint8Array
import app.witness.core.verify.web.BridgeVerifyEngine
import app.witness.core.verify.web.DimensionsJs
import app.witness.core.verify.web.dimensions
import kotlinx.coroutines.await

actual fun createVerifyEngine(config: VerifyConfig): VerifyEngine = BridgeVerifyEngine(config.engineUrl)

actual fun platformImageInfoReader(): ImageInfoReader = SniffingImageInfoReader { bytes ->
    dimensions(bytes.toUint8Array()).await<DimensionsJs?>()?.let { it.width to it.height }
}
