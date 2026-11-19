package app.witness.core.icp

import app.witness.core.icp.web.BridgeIcp
import app.witness.core.icp.web.WebCaseCrypto

actual fun createIcpServices(endpoint: IcpEndpoint): IcpServices {
    val bridge = BridgeIcp(endpoint)
    return IcpServices(registry = bridge, index = bridge, cases = bridge, crypto = WebCaseCrypto(), uploader = null)
}
