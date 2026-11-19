package app.witness.core.icp

data class IcpEndpoint(val host: String, val fetchRootKey: Boolean, val registryId: String, val caRoot: String = "") {
    companion object {
        const val LOCAL_HOST: String = "http://127.0.0.1:4943"
        const val MAINNET_HOST: String = "https://icp-api.io"

        fun localReplica(registryId: String): IcpEndpoint = IcpEndpoint(LOCAL_HOST, fetchRootKey = true, registryId)

        fun mainnet(registryId: String): IcpEndpoint = IcpEndpoint(MAINNET_HOST, fetchRootKey = false, registryId)
    }
}
