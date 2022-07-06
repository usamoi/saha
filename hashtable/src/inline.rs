use std::alloc::Layout;

#[derive(Clone)]
struct Slot<const N: usize> {
    key: [u64; N],
    val: u64,
}

static_assertions::assert_eq_size!(Slot<1>, [u64; 2]);
static_assertions::assert_eq_size!(Slot<2>, [u64; 3]);
static_assertions::assert_eq_size!(Slot<3>, [u64; 4]);

impl<const N: usize> Slot<N> {
    fn exists(&self) -> bool {
        self.key[N - 1] != u64::MAX
    }
}

pub struct InlineHashtable<const N: usize> {
    slots: Box<[Slot<N>]>,
    len: usize,
}

impl<const N: usize> InlineHashtable<N> {
    pub fn new() -> Self {
        Self::with_capacity(1 << 8)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: unsafe {
                let layout = Layout::array::<Slot<N>>(capacity).unwrap();
                let data = std::alloc::alloc(layout) as *mut Slot<N>;
                (data as *mut u8).write_bytes(u8::MAX, layout.size());
                Box::from_raw(std::ptr::slice_from_raw_parts_mut(data, capacity))
            },
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    pub fn get(&self, key: [u64; N], hash: u64) -> Option<u64> {
        debug_assert_ne!(key[N - 1], u64::MAX);
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                return None;
            }
            if self.slots[i].key == key {
                return Some(self.slots[i].val);
            }
        }
        None
    }
    pub fn get_mut(&mut self, key: [u64; N], hash: u64) -> Option<&mut u64> {
        debug_assert_ne!(key[N - 1], u64::MAX);
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                return None;
            }
            if self.slots[i].key == key {
                return Some(&mut self.slots[i].val);
            }
        }
        None
    }
    pub fn insert(&mut self, key: [u64; N], hash: u64) -> Result<&mut u64, Option<&mut u64>> {
        debug_assert_ne!(key[N - 1], u64::MAX);
        let index = (hash as usize) & (self.slots.len() - 1);
        for i in (index..self.slots.len()).chain(0..index) {
            if !self.slots[i].exists() {
                self.len += 1;
                self.slots[i].key = key;
                return Ok(&mut self.slots[i].val);
            }
            if self.slots[i].key == key {
                return Err(Some(&mut self.slots[i].val));
            }
        }
        Err(None)
    }
    pub fn grow<H: Fn(&[u64]) -> u64>(&mut self, hasher: H) {
        let old_capacity = self.slots.len();
        let new_capacity = self.slots.len() * 2;
        let old_layout = Layout::for_value(self.slots.as_ref());
        let new_layout = Layout::array::<Slot<N>>(new_capacity).unwrap();
        unsafe {
            let data = std::alloc::realloc(
                self.slots.as_mut_ptr() as *mut u8,
                old_layout,
                new_layout.size(),
            );
            data.add(old_layout.size())
                .write_bytes(u8::MAX, new_layout.size() - old_layout.size());
            let src = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                data as *mut Slot<N>,
                new_capacity,
            ));
            std::ptr::write(&mut self.slots, src);
        };
        for i in 0..old_capacity {
            let slot = self.slots[i].clone();
            if !slot.exists() {
                continue;
            }
            let index = (hasher(&slot.key) as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if !self.slots[j].exists() {
                    self.slots[i].key = [u64::MAX; N];
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
            let index = (hasher(&slot.key) as usize) & (self.slots.len() - 1);
            for j in (index..self.slots.len()).chain(0..index) {
                if j == i {
                    break;
                }
                if !self.slots[j].exists() {
                    self.slots[i].key = [u64::MAX; N];
                    self.slots[j] = slot;
                    break;
                }
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = ([u64; N], u64)> + '_ {
        self.slots
            .iter()
            .filter(|&slot| slot.exists())
            .map(|slot| (slot.key, slot.val))
    }
}
