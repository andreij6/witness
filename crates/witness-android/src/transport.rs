use crate::error::WitnessError;
use candid::Principal;
use futures::future::BoxFuture;
use ic_agent::Agent;
use ic_agent::identity::BasicIdentity;
use tokio::sync::OnceCell;
use witness_proof::IC_ROOT_KEY_DER;

pub(crate) type Reply<'a> = BoxFuture<'a, Result<Vec<u8>, WitnessError>>;

pub(crate) trait Transport: Send + Sync {
    fn query<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a>;
    fn update<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a>;
    fn root_key(&self) -> Reply<'_>;
}

pub(crate) struct AgentTransport {
    agent: Agent,
    local: bool,
    root_key: OnceCell<()>,
}

pub(crate) fn identity_seed(seed: Option<&[u8]>) -> Result<Option<[u8; 32]>, WitnessError> {
    seed.map(|s| {
        <[u8; 32]>::try_from(s).map_err(|_| WitnessError::invalid("identity seed must be 32 bytes"))
    })
    .transpose()
}

impl AgentTransport {
    pub(crate) fn new(
        url: &str,
        local: bool,
        seed: Option<[u8; 32]>,
    ) -> Result<Self, WitnessError> {
        let parsed =
            url::Url::parse(url).map_err(|e| WitnessError::invalid(format!("replica url: {e}")))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
            return Err(WitnessError::invalid(
                "replica url must be http(s) with a host",
            ));
        }
        if !local && parsed.scheme() != "https" {
            return Err(WitnessError::invalid(
                "replica url must be https outside local builds",
            ));
        }
        let builder = Agent::builder().with_url(parsed.as_str());
        let builder = match seed {
            Some(s) => builder.with_identity(BasicIdentity::from_raw_key(&s)),
            None => builder,
        };
        let agent = builder
            .build()
            .map_err(|e| WitnessError::invalid(format!("replica url: {e}")))?;
        Ok(Self {
            agent,
            local,
            root_key: OnceCell::new(),
        })
    }

    async fn ready(&self) -> Result<(), WitnessError> {
        if !self.local {
            return Ok(());
        }
        self.root_key
            .get_or_try_init(|| async { self.agent.fetch_root_key().await })
            .await
            .map(|_| ())
            .map_err(WitnessError::network)
    }
}

impl Transport for AgentTransport {
    fn query<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a> {
        Box::pin(async move {
            self.ready().await?;
            self.agent
                .query(&canister, method)
                .with_arg(arg)
                .call()
                .await
                .map_err(WitnessError::network)
        })
    }

    fn update<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a> {
        Box::pin(async move {
            self.ready().await?;
            self.agent
                .update(&canister, method)
                .with_arg(arg)
                .call_and_wait()
                .await
                .map_err(WitnessError::network)
        })
    }

    fn root_key(&self) -> Reply<'_> {
        Box::pin(async move {
            if !self.local {
                return Ok(IC_ROOT_KEY_DER.to_vec());
            }
            self.ready().await?;
            Ok(self.agent.read_root_key())
        })
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    pub(crate) type Handler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WitnessError> + Send + Sync>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct Call {
        pub canister: Principal,
        pub method: String,
        pub update: bool,
        pub arg: Vec<u8>,
    }

    #[derive(Default)]
    pub(crate) struct Mock {
        handlers: BTreeMap<(Principal, String), Handler>,
        pub calls: Mutex<Vec<Call>>,
        root_key: Option<Vec<u8>>,
    }

    impl Mock {
        pub(crate) fn with_root_key(mut self, der: Vec<u8>) -> Self {
            self.root_key = Some(der);
            self
        }

        pub(crate) fn on(
            mut self,
            canister: Principal,
            method: &str,
            f: impl Fn(&[u8]) -> Result<Vec<u8>, WitnessError> + Send + Sync + 'static,
        ) -> Self {
            self.handlers.insert((canister, method.into()), Box::new(f));
            self
        }

        pub(crate) fn calls(&self) -> Vec<Call> {
            self.calls.lock().unwrap().clone()
        }

        pub(crate) fn methods(&self) -> Vec<String> {
            self.calls().into_iter().map(|c| c.method).collect()
        }

        fn dispatch(
            &self,
            canister: Principal,
            method: &str,
            arg: Vec<u8>,
            update: bool,
        ) -> Result<Vec<u8>, WitnessError> {
            let reply = match self.handlers.get(&(canister, method.to_owned())) {
                Some(h) => h(&arg),
                None => Err(WitnessError::network(format!(
                    "no route {canister} {method}"
                ))),
            };
            self.calls.lock().unwrap().push(Call {
                canister,
                method: method.into(),
                update,
                arg,
            });
            reply
        }
    }

    impl Transport for Mock {
        fn query<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a> {
            Box::pin(async move { self.dispatch(canister, method, arg, false) })
        }

        fn update<'a>(&'a self, canister: Principal, method: &'a str, arg: Vec<u8>) -> Reply<'a> {
            Box::pin(async move { self.dispatch(canister, method, arg, true) })
        }

        fn root_key(&self) -> Reply<'_> {
            let key = self
                .root_key
                .clone()
                .ok_or_else(|| WitnessError::network("no root key"));
            Box::pin(async move { key })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_seed_must_be_32_bytes() {
        assert_eq!(identity_seed(None).unwrap(), None);
        assert_eq!(identity_seed(Some(&[1; 32])).unwrap(), Some([1; 32]));
        for len in [0, 31, 33] {
            assert!(identity_seed(Some(&vec![0; len])).is_err());
        }
    }

    #[test]
    fn agent_transport_rejects_bad_urls() {
        for bad in [
            "not a url",
            "",
            "ftp://10.0.2.2:4943",
            "file:///tmp/x",
            "http://",
        ] {
            assert!(
                matches!(
                    AgentTransport::new(bad, true, None),
                    Err(WitnessError::InvalidInput { .. })
                ),
                "{bad}"
            );
        }
    }

    #[test]
    fn agent_transport_builds_without_network() {
        assert!(AgentTransport::new("http://10.0.2.2:4943", true, Some([7; 32])).is_ok());
        assert!(AgentTransport::new("https://icp-api.io", false, None).is_ok());
    }

    #[test]
    fn unreachable_replica_is_a_network_error() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let t = AgentTransport::new("http://127.0.0.1:9", true, None).unwrap();
        let err = rt
            .block_on(t.query(Principal::anonymous(), "status", vec![]))
            .unwrap_err();
        assert!(matches!(err, WitnessError::Network { .. }));
        let err = rt
            .block_on(t.update(Principal::anonymous(), "submit", vec![]))
            .unwrap_err();
        assert!(matches!(err, WitnessError::Network { .. }));
        assert!(matches!(
            rt.block_on(t.root_key()),
            Err(WitnessError::Network { .. })
        ));
    }

    #[test]
    fn production_root_key_is_the_pinned_ic_key_without_network() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let t = AgentTransport::new("https://127.0.0.1:9", false, None).unwrap();
        assert_eq!(rt.block_on(t.root_key()).unwrap(), IC_ROOT_KEY_DER.to_vec());
    }

    #[test]
    fn production_refuses_cleartext_replica_urls() {
        for url in [
            "http://icp-api.io",
            "http://10.0.2.2:4943",
            "HTTP://icp-api.io",
            "http://127.0.0.1:8090",
        ] {
            assert!(
                matches!(
                    AgentTransport::new(url, false, Some([1; 32])),
                    Err(WitnessError::InvalidInput { .. })
                ),
                "{url}"
            );
            assert!(AgentTransport::new(url, true, None).is_ok(), "{url}");
        }
        assert!(AgentTransport::new("HTTPS://icp-api.io", false, None).is_ok());
    }
}
