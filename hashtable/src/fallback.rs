use bumpalo::Bump;
use std::alloc::Layout;
use std::ptr::NonNull;

#[derive(Clone)]
#[repr(align(32))]
struct Slot {
    key: Option<NonNull<[u8]>>,
    hash: u64,
    val: u64,
}

impl Slot {
    fn exists(&self) -> bool {
        self.key.is_some()
    }
    unsafe fn key(&self) -> &[u8] {
        self.key.unwrap().as_mut()
    }
}

static_assertions::assert_eq_size!(Slot, [u8; 32]);

pub struct FallbackHashtable {
    slots: Box<[Slot]>,
    arena: Bump,
    len: usize,
}

impl FallbackHashtable {
    pub fn new() -> Self {
        Self::with_capacity(1 << 8)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: unsafe {
                let layout = Layout::array::<Slot>(capacity).unwrap();
                let data = std::alloc::alloc_zeroed(layout) as *mut Slot;
                Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, capacity))
            },
            arena: Bump::new(),
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    pub fn get(&self, key: &[u8], hash: u64) -> Option<u64> {
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                return None;
            }
            if self.slots[i].hash == hash && unsafe { self.slots[i].key() } == key {
                return Some(self.slots[i].val);
            }
        }
        None
    }
    pub fn get_mut(&mut self, key: &[u8], hash: u64) -> Option<&mut u64> {
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                return None;
            }
            if self.slots[i].hash == hash && unsafe { self.slots[i].key() } == key {
                return Some(&mut self.slots[i].val);
            }
        }
        None
    }
    pub fn insert(&mut self, key: &[u8], hash: u64) -> Result<&mut u64, Option<&mut u64>> {
        let saved = NonNull::new(self.arena.alloc_slice_copy(key) as *mut [u8]);
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                self.len += 1;
                self.slots[i].key = saved;
                self.slots[i].hash = hash;
                return Ok(&mut self.slots[i].val);
            }
            if self.slots[i].hash == hash && unsafe { self.slots[i].key() } == key {
                return Err(Some(&mut self.slots[i].val));
            }
        }
        Err(None)
    }
    pub fn grow(&mut self) {
        let old_capacity = self.slots.len();
        let new_capacity = self.slots.len() * 2;
        let old_layout = Layout::for_value(self.slots.as_ref());
        let new_layout = Layout::array::<Slot>(new_capacity).unwrap();
        unsafe {
            let data = std::alloc::realloc(
                self.slots.as_mut_ptr() as *mut u8,
                old_layout,
                new_layout.size(),
            );
            data.add(old_layout.size())
                .write_bytes(0, new_layout.size() - old_layout.size());
            let src = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                data as *mut Slot,
                new_capacity,
            ));
            std::ptr::write(&mut self.slots, src);
        };
        for i in 0..old_capacity {
            let slot = self.slots[i].clone();
            if !slot.exists() {
                continue;
            }
            let index = (slot.hash as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if !self.slots[j].exists() {
                    self.slots[i].key = None;
                    self.slots[j] = slot;
                    break;
                }
            }
        }
        for i in old_capacity..new_capacity {
            let slot = self.slots[i].clone();
            if !slot.exists() {
                break;
            }
            let index = (slot.hash as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if !self.slots[j].exists() {
                    self.slots[i].key = None;
                    self.slots[j] = slot;
                    break;
                }
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&[u8], u64)> {
        self.slots
            .iter()
            .filter(|&slot| slot.exists())
            .map(|slot| (unsafe { slot.key() }, slot.val))
    }
}
