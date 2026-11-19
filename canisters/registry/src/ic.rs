use crate::config::{
    BOOTSTRAP_RETRY, DRAIN_PERIOD, OUTBOX_BATCH, OUTBOX_SCAN, OUTBOX_TOTAL, PUBLISH_PERIOD,
    SCHNORR_KEY,
};
use crate::outbox::{Budget, complete, plan, resolve};
use crate::proof::Roots;
use crate::{ca, status, store};
use core::time::Duration;
use ic_cdk::call::Call;
use ic_cdk_management_canister::{
    SchnorrAlgorithm, SchnorrKeyId, SchnorrPublicKeyArgs, SignWithSchnorrArgs, raw_rand,
    schnorr_public_key, sign_with_schnorr,
};
use serde_bytes::ByteBuf;
use std::cell::Cell;
use witness_canister_common::{TimerSpec, runtime};

pub static TIMERS: [TimerSpec; 2] = [
    TimerSpec {
        name: "drain",
        period: DRAIN_PERIOD,
        tick: || Box::pin(drain()),
    },
    TimerSpec {
        name: "publish",
        period: PUBLISH_PERIOD,
        tick: || Box::pin(publish()),
    },
];

pub fn certify(roots: &Roots) {
    runtime::set_certified_root(&roots.certified());
}

fn key_id() -> SchnorrKeyId {
    SchnorrKeyId {
        algorithm: SchnorrAlgorithm::Ed25519,
        name: SCHNORR_KEY.to_owned(),
    }
}

pub async fn sign(message: Vec<u8>) -> Result<Vec<u8>, String> {
    let args = SignWithSchnorrArgs {
        message,
        derivation_path: vec![],
        key_id: key_id(),
        aux: None,
    };
    sign_with_schnorr(&args)
        .await
        .map(|r| r.signature)
        .map_err(|e| format!("sign_with_schnorr: {e}"))
}

async fn bootstrap() -> Result<(), String> {
    if store::with(|s| s.state.get().secret().is_none()) {
        let r = raw_rand().await.map_err(|e| format!("raw_rand: {e}"))?;
        store::with(|s| {
            s.update_state(|st| {
                st.secret.get_or_insert_with(|| ByteBuf::from(r));
            })
        });
    }
    if store::with(|s| s.state.get().ca_root().is_none()) {
        let pk = schnorr_public_key(&SchnorrPublicKeyArgs {
            canister_id: None,
            derivation_path: vec![],
            key_id: key_id(),
        })
        .await
        .map_err(|e| format!("schnorr_public_key: {e}"))?
        .public_key;
        let tbs = ca::root_to_sign(ic_cdk::api::time(), &pk)?;
        let sig = sign(tbs.clone()).await?;
        let der = ca::finish_root(&tbs, &sig, &pk)?;
        store::with(|s| {
            s.update_state(|st| {
                st.ca_root.get_or_insert_with(|| ByteBuf::from(der));
            })
        });
    }
    Ok(())
}

pub fn schedule_bootstrap(delay: Duration) {
    ic_cdk_timers::set_timer(delay, async {
        if let Err(e) = bootstrap().await {
            ic_cdk::println!("bootstrap failed: {e}");
            schedule_bootstrap(BOOTSTRAP_RETRY);
        }
    });
}

thread_local! {
    static DRAIN_CURSOR: Cell<u64> = const { Cell::new(0) };
}

const DRAIN_BUDGET: Budget = Budget {
    per_dest: OUTBOX_SCAN,
    total: OUTBOX_TOTAL,
    batch: OUTBOX_BATCH,
};

async fn drain() {
    let p = store::with(|s| {
        let p = plan(s, DRAIN_CURSOR.get(), DRAIN_BUDGET);
        complete(s, &p.dropped);
        p
    });
    DRAIN_CURSOR.set(p.resume);
    for b in p.batches {
        let Some(target) = store::with(|s| resolve(s, b.dest)) else {
            continue;
        };
        let sent = Call::bounded_wait(target, b.call.method())
            .take_raw_args(b.call.arg())
            .await;
        if sent.is_ok() {
            store::with(|s| complete(s, &b.keys));
        }
    }
}

async fn publish() {
    store::with(|s| status::publish(s, ic_cdk::api::time()));
}
