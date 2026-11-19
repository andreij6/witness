use ic_stable_structures::DefaultMemoryImpl;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use std::cell::RefCell;

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

pub fn memory(id: MemoryId) -> Memory {
    MANAGER.with_borrow(|m| m.get(id))
}

pub const CONFIG: MemoryId = MemoryId::new(0);
pub const COMMON_RESERVED_END: u8 = 7;
pub const FIRST_FREE: u8 = 32;
pub const LAST_ID: u8 = 254;

pub mod registry {
    use super::MemoryId;
    pub const MMR_NODES: MemoryId = MemoryId::new(8);
    pub const MMR_PEAKS: MemoryId = MemoryId::new(9);
    pub const SMT_NODES: MemoryId = MemoryId::new(10);
    pub const RECORDS: MemoryId = MemoryId::new(11);
    pub const RECORD_BY_ID: MemoryId = MemoryId::new(12);
    pub const KEYS: MemoryId = MemoryId::new(13);
    pub const PENDING_KEYS: MemoryId = MemoryId::new(14);
    pub const COUNTERS: MemoryId = MemoryId::new(15);
    pub const BY_CAPTURE: MemoryId = MemoryId::new(16);
    pub const DERIVATIVES: MemoryId = MemoryId::new(17);
    pub const TOMBSTONES: MemoryId = MemoryId::new(18);
    pub const OUTBOX: MemoryId = MemoryId::new(19);
    pub const QUOTAS: MemoryId = MemoryId::new(20);
    pub const REVOCATIONS: MemoryId = MemoryId::new(21);
    pub const II_LINKS: MemoryId = MemoryId::new(22);
    pub const STATE: MemoryId = MemoryId::new(23);
    pub const ALL: &[MemoryId] = &[
        MMR_NODES,
        MMR_PEAKS,
        SMT_NODES,
        RECORDS,
        RECORD_BY_ID,
        KEYS,
        PENDING_KEYS,
        COUNTERS,
        BY_CAPTURE,
        DERIVATIVES,
        TOMBSTONES,
        OUTBOX,
        QUOTAS,
        REVOCATIONS,
        II_LINKS,
        STATE,
    ];
}

pub mod index {
    use super::MemoryId;
    pub const PDQ_ROWS: MemoryId = MemoryId::new(8);
    pub const GRIDS: MemoryId = MemoryId::new(9);
    pub const POSTINGS: MemoryId = MemoryId::new(10);
    pub const WATERMARK: MemoryId = MemoryId::new(11);
    pub const EXACT_ASSET: MemoryId = MemoryId::new(12);
    pub const EXACT_PIXEL: MemoryId = MemoryId::new(13);
    pub const DERIVATIVES: MemoryId = MemoryId::new(14);
    pub const LOCAL_TO_RECORD: MemoryId = MemoryId::new(15);
    pub const RECORD_TO_LOCAL: MemoryId = MemoryId::new(16);
    pub const STATE: MemoryId = MemoryId::new(17);
    pub const ALL: &[MemoryId] = &[
        PDQ_ROWS,
        GRIDS,
        POSTINGS,
        WATERMARK,
        EXACT_ASSET,
        EXACT_PIXEL,
        DERIVATIVES,
        LOCAL_TO_RECORD,
        RECORD_TO_LOCAL,
        STATE,
    ];
}

pub mod pages {
    use super::MemoryId;
    pub const CERT_TREE: MemoryId = MemoryId::new(8);
    pub const SHELLS: MemoryId = MemoryId::new(9);
    pub const STATE: MemoryId = MemoryId::new(10);
    pub const ALL: &[MemoryId] = &[CERT_TREE, SHELLS, STATE];
}

pub mod media {
    use super::MemoryId;
    pub const BLOBS: MemoryId = MemoryId::new(8);
    pub const BLOB_INDEX: MemoryId = MemoryId::new(9);
    pub const UPLOADS: MemoryId = MemoryId::new(10);
    pub const QUOTAS: MemoryId = MemoryId::new(11);
    pub const GRANT_USAGE: MemoryId = MemoryId::new(12);
    pub const EXPIRY: MemoryId = MemoryId::new(13);
    pub const CERT_TREE: MemoryId = MemoryId::new(14);
    pub const STATE: MemoryId = MemoryId::new(15);
    pub const ALL: &[MemoryId] = &[
        BLOBS,
        BLOB_INDEX,
        UPLOADS,
        QUOTAS,
        GRANT_USAGE,
        EXPIRY,
        CERT_TREE,
        STATE,
    ];
}

pub const fn local(offset: u8) -> MemoryId {
    let id = FIRST_FREE as u16 + offset as u16;
    assert!(id <= LAST_ID as u16, "memory id out of range");
    MemoryId::new(id as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_stable_structures::Memory as _;

    fn raw(id: MemoryId) -> u8 {
        let mut probe = [0u8; 1];
        let m = memory(id);
        if m.size() == 0 {
            m.grow(1);
        }
        m.read(0, &mut probe);
        probe[0]
    }

    fn check_table(ids: &[MemoryId]) {
        let mut seen: Vec<MemoryId> = ids.to_vec();
        seen.push(CONFIG);
        seen.sort();
        let n = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), n, "duplicate memory id");
        for id in ids {
            let mut found = false;
            for v in COMMON_RESERVED_END + 1..FIRST_FREE {
                found |= *id == MemoryId::new(v);
            }
            assert!(found, "{id:?} outside canister range");
        }
    }

    #[test]
    fn tables_are_unique_and_in_range() {
        check_table(registry::ALL);
        check_table(index::ALL);
        check_table(pages::ALL);
        check_table(media::ALL);
    }

    #[test]
    fn tables_are_dense_from_first_canister_id() {
        for table in [registry::ALL, index::ALL, pages::ALL, media::ALL] {
            for (i, id) in table.iter().enumerate() {
                assert_eq!(*id, MemoryId::new(COMMON_RESERVED_END + 1 + i as u8));
            }
        }
    }

    #[test]
    fn local_ids_start_after_named_ids() {
        assert_eq!(local(0), MemoryId::new(FIRST_FREE));
        assert_eq!(local(LAST_ID - FIRST_FREE), MemoryId::new(LAST_ID));
    }

    #[test]
    #[should_panic(expected = "memory id out of range")]
    fn local_rejects_overflow() {
        local(LAST_ID - FIRST_FREE + 1);
    }

    #[test]
    #[should_panic(expected = "memory id out of range")]
    fn local_rejects_u8_max() {
        local(u8::MAX);
    }

    #[test]
    fn virtual_memories_are_isolated() {
        let a = memory(registry::RECORDS);
        let b = memory(registry::KEYS);
        a.grow(1);
        b.grow(1);
        a.write(0, &[0xAA]);
        b.write(0, &[0x55]);
        assert_eq!(raw(registry::RECORDS), 0xAA);
        assert_eq!(raw(registry::KEYS), 0x55);
    }

    #[test]
    fn same_id_returns_same_memory() {
        let a = memory(local(5));
        a.grow(1);
        a.write(7, &[9, 8, 7]);
        let mut out = [0u8; 3];
        memory(local(5)).read(7, &mut out);
        assert_eq!(out, [9, 8, 7]);
    }

    #[test]
    fn fresh_memory_is_empty() {
        assert_eq!(memory(local(200)).size(), 0);
    }
}
