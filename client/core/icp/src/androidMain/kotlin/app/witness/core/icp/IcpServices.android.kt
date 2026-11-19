package app.witness.core.icp

import app.witness.core.icp.fake.InMemoryCaseStore
import app.witness.core.icp.fake.InMemoryIndex
import app.witness.core.icp.fake.InMemoryRegistry
import app.witness.core.icp.index.FanOutIndexSearch

actual fun createIcpServices(endpoint: IcpEndpoint): IcpServices {
    val cases = InMemoryCaseStore()
    return IcpServices(
        registry = InMemoryRegistry(),
        index = FanOutIndexSearch(InMemoryIndex()),
        cases = cases,
        crypto = JcaCaseCrypto(),
        uploader = cases,
    )
}
