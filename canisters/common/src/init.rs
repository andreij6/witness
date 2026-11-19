use candid::{CandidType, Decode, Encode, Principal};
use ic_stable_structures::Storable;
use ic_stable_structures::storable::Bound;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistryInit {
    pub index: Vec<Principal>,
    pub media: Vec<Principal>,
    pub pages: Principal,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerInit {
    pub registry: Principal,
    pub shard: u8,
    pub origins: Option<Origins>,
}

#[derive(CandidType, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origins {
    pub site: String,
    pub case: String,
    pub app: String,
    pub api: String,
    pub canister: String,
}

impl Default for Origins {
    fn default() -> Self {
        Self {
            site: "https://witness.app".into(),
            case: "https://case.witness.app".into(),
            app: "https://app.witness.app".into(),
            api: "https://icp-api.io".into(),
            canister: "https://*.icp0.io".into(),
        }
    }
}

fn host_ok(host: &str, wildcard: bool) -> bool {
    let host = if wildcard {
        match host.strip_prefix("*.") {
            Some(h) => h,
            None => return false,
        }
    } else {
        host
    };
    let (name, port) = host.split_once(':').unwrap_or((host, ""));
    let port_ok = port.is_empty() && !host.ends_with(':')
        || (1..=5).contains(&port.len()) && port.bytes().all(|b| b.is_ascii_digit());
    port_ok
        && !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|l| {
            !l.is_empty()
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

pub fn origin_ok(origin: &str, wildcard: bool) -> bool {
    origin
        .strip_prefix("https://")
        .or_else(|| origin.strip_prefix("http://"))
        .is_some_and(|h| host_ok(h, wildcard))
}

impl Origins {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, wildcard) in [
            ("site", &self.site, false),
            ("case", &self.case, false),
            ("app", &self.app, false),
            ("api", &self.api, false),
            ("canister", &self.canister, true),
        ] {
            if !origin_ok(value, wildcard) {
                return Err(format!("origins.{name}: {value:?} is not a bare origin"));
            }
        }
        Ok(())
    }

    pub fn is_secure(&self) -> bool {
        [&self.site, &self.case, &self.app, &self.api, &self.canister]
            .iter()
            .all(|o| o.starts_with("https://"))
    }

    pub fn canister_url(&self, canister: &str) -> String {
        self.canister.replacen('*', canister, 1)
    }

    pub fn host(origin: &str) -> &str {
        origin.split_once("://").map_or(origin, |(_, h)| h)
    }

    pub fn browser_origins(&self) -> [&str; 3] {
        [&self.site, &self.case, &self.app]
    }
}

impl Default for RegistryInit {
    fn default() -> Self {
        Self {
            index: Vec::new(),
            media: Vec::new(),
            pages: Principal::anonymous(),
        }
    }
}

impl Default for PeerInit {
    fn default() -> Self {
        Self {
            registry: Principal::anonymous(),
            shard: 0,
            origins: None,
        }
    }
}

impl RegistryInit {
    pub fn is_configured(&self) -> bool {
        self.pages != Principal::anonymous()
            && !self.index.is_empty()
            && !self.media.is_empty()
            && self
                .index
                .iter()
                .chain(&self.media)
                .all(|p| *p != Principal::anonymous())
    }
}

impl PeerInit {
    pub fn is_configured(&self) -> bool {
        self.registry != Principal::anonymous()
    }

    pub fn origins(&self) -> Origins {
        self.origins.clone().unwrap_or_default()
    }
}

macro_rules! candid_storable {
    ($t:ty) => {
        impl Storable for $t {
            fn to_bytes(&self) -> Cow<'_, [u8]> {
                Cow::Owned(Encode!(self).expect("candid encode"))
            }

            fn into_bytes(self) -> Vec<u8> {
                Encode!(&self).expect("candid encode")
            }

            fn from_bytes(bytes: Cow<[u8]>) -> Self {
                Decode!(bytes.as_ref(), Self).expect("candid decode")
            }

            const BOUND: Bound = Bound::Unbounded;
        }
    };
}

candid_storable!(RegistryInit);
candid_storable!(PeerInit);

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: u8) -> Principal {
        Principal::from_slice(&[n; 10])
    }

    fn registry_init() -> RegistryInit {
        RegistryInit {
            index: vec![p(1)],
            media: vec![p(2), p(3)],
            pages: p(4),
        }
    }

    #[test]
    fn defaults_are_unconfigured() {
        assert!(!RegistryInit::default().is_configured());
        assert!(!PeerInit::default().is_configured());
    }

    #[test]
    fn configured_when_every_peer_is_set() {
        assert!(registry_init().is_configured());
        assert!(
            PeerInit {
                registry: p(9),
                shard: 3,
                origins: None,
            }
            .is_configured()
        );
    }

    #[test]
    fn anonymous_or_missing_peers_are_unconfigured() {
        let mut r = registry_init();
        r.media.push(Principal::anonymous());
        assert!(!r.is_configured());
        let mut r = registry_init();
        r.index.clear();
        assert!(!r.is_configured());
        let mut r = registry_init();
        r.media.clear();
        assert!(!r.is_configured());
        let mut r = registry_init();
        r.pages = Principal::anonymous();
        assert!(!r.is_configured());
    }

    #[test]
    fn storable_round_trips() {
        let r = registry_init();
        assert_eq!(RegistryInit::from_bytes(r.to_bytes()), r);
        assert_eq!(
            RegistryInit::from_bytes(Cow::Owned(r.clone().into_bytes())),
            r
        );
        let peer = PeerInit {
            registry: p(7),
            shard: 255,
            origins: Some(local()),
        };
        assert_eq!(PeerInit::from_bytes(peer.to_bytes()), peer);
        assert_eq!(
            PeerInit::from_bytes(Cow::Owned(peer.clone().into_bytes())),
            peer
        );
    }

    #[test]
    fn storable_is_deterministic() {
        let r = registry_init();
        let bytes = r.to_bytes().into_owned();
        assert_eq!(bytes, r.clone().to_bytes().into_owned());
        assert_eq!(bytes, r.into_bytes());
    }

    fn local() -> Origins {
        Origins {
            site: "http://pages-id.localhost:8090".into(),
            case: "http://pages-id.localhost:8090".into(),
            app: "http://web-id.localhost:8090".into(),
            api: "http://localhost:8090".into(),
            canister: "http://*.localhost:8090".into(),
        }
    }

    #[derive(CandidType)]
    struct LegacyPeerInit {
        registry: Principal,
        shard: u8,
    }

    #[test]
    fn peer_init_stored_before_origins_still_decodes() {
        let old = Encode!(&LegacyPeerInit {
            registry: p(5),
            shard: 2
        })
        .unwrap();
        let got = PeerInit::from_bytes(Cow::Owned(old));
        assert_eq!(got.registry, p(5));
        assert_eq!(got.shard, 2);
        assert_eq!(got.origins, None);
        assert_eq!(got.origins(), Origins::default());
    }

    #[test]
    fn production_origins_are_the_default_and_valid() {
        let o = Origins::default();
        assert_eq!(o.validate(), Ok(()));
        assert!(o.is_secure());
        assert_eq!(
            o.browser_origins(),
            [
                "https://witness.app",
                "https://case.witness.app",
                "https://app.witness.app"
            ]
        );
        assert_eq!(o.canister_url("aaaaa-aa"), "https://aaaaa-aa.icp0.io");
        assert_eq!(Origins::host(&o.case), "case.witness.app");
        assert_eq!(PeerInit::default().origins(), o);
    }

    #[test]
    fn local_origins_are_valid_but_not_secure() {
        let o = local();
        assert_eq!(o.validate(), Ok(()));
        assert!(!o.is_secure());
        assert_eq!(
            o.canister_url("4xhad-gd777-77775-aaacq-cai"),
            "http://4xhad-gd777-77775-aaacq-cai.localhost:8090"
        );
        assert_eq!(Origins::host(&o.api), "localhost:8090");
    }

    #[test]
    fn origins_reject_anything_but_bare_origins() {
        for bad in [
            "",
            "witness.app",
            "ftp://witness.app",
            "https://",
            "https://witness.app/",
            "https://witness.app/path",
            "https://Witness.app",
            "https://witness..app",
            "https://witness.app:",
            "https://witness.app:123456",
            "https://witness.app:80a",
            "https://wit ness.app",
            "https://witness.app\"><script>",
            "https://*.icp0.io",
            "https://user@witness.app",
            "https://witness.app?x",
        ] {
            let o = Origins {
                app: bad.into(),
                ..Origins::default()
            };
            assert!(o.validate().is_err(), "{bad:?}");
        }
        for bad in [
            "https://icp0.io",
            "https://*icp0.io",
            "https://a.*.icp0.io",
            "https://*.",
        ] {
            let o = Origins {
                canister: bad.into(),
                ..Origins::default()
            };
            assert!(o.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    #[should_panic(expected = "candid decode")]
    fn storable_rejects_garbage() {
        PeerInit::from_bytes(Cow::Borrowed(b"not candid"));
    }

    #[test]
    #[should_panic(expected = "candid decode")]
    fn storable_rejects_other_type() {
        let bytes = registry_init().into_bytes();
        PeerInit::from_bytes(Cow::Owned(bytes));
    }
}
