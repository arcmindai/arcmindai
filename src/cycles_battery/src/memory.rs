use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::DefaultMemoryImpl;
use std::cell::RefCell;

// A memory for upgrades, where data from the heap can be serialized/deserialized.
const UPGRADES: MemoryId = MemoryId::new(0);

// A memory for the StableVec we're using. A new memory should be created for
// every additional stable structure.
const STABLE_TOPUP_VEC: MemoryId = MemoryId::new(1);
const STABLE_CYCLES_MONITOR_VEC: MemoryId = MemoryId::new(2);

pub type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    // The memory manager is used for simulating multiple memories. Given a `MemoryId` it can
    // return a memory that can be used by stable structures.
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}

pub fn get_upgrades_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.borrow().get(UPGRADES))
}

pub fn get_stable_topup_vec_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.borrow().get(STABLE_TOPUP_VEC))
}

pub fn get_stable_cycles_monitor_vec_memory() -> Memory {
    MEMORY_MANAGER.with(|m| m.borrow().get(STABLE_CYCLES_MONITOR_VEC))
}
