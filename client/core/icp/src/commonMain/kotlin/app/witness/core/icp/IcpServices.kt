package app.witness.core.icp

import app.witness.core.domain.Clock
import app.witness.core.icp.cases.CaseCrypto
import app.witness.core.icp.cases.CaseOpener
import app.witness.core.icp.cases.CaseSealer
import app.witness.core.icp.cases.CaseStore
import app.witness.core.icp.cases.CaseUploader
import app.witness.core.icp.index.CandidateSearch
import app.witness.core.icp.index.IndexSearch
import app.witness.core.icp.registry.RegistryGateway

class IcpServices(
    val registry: RegistryGateway,
    val index: IndexSearch,
    val cases: CaseStore,
    val crypto: CaseCrypto,
    val uploader: CaseUploader?,
) {
    val candidates: CandidateSearch by lazy { CandidateSearch(registry, index) }

    val sealer: CaseSealer by lazy { CaseSealer(crypto) }

    fun opener(clock: Clock): CaseOpener = CaseOpener(cases, crypto, clock)
}

expect fun createIcpServices(endpoint: IcpEndpoint): IcpServices
