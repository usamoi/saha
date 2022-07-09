use common_hashtable::HashMap as DatabendMap;
use common_hashtable::HashTableEntity;
use common_hashtable::UnsafeBytesRef;
use hashbrown::hash_map::EntryRef;
use hashbrown::HashMap as HashbrownMap;
use hashtable::adaptive_hashtable::AdaptiveHashtable;

pub trait Subject {
    const NAME: &'static str;
    fn new() -> Self;
    fn build(&mut self, key: Box<[u8]>, insert: impl FnMut() -> u64, update: impl FnMut(&mut u64));
    fn probe(&self, key: &Box<[u8]>) -> Option<u64>;
    fn foreach<F: FnMut((&[u8], u64))>(&self, f: F);
}

impl Subject for HashbrownMap<Box<[u8]>, u64> {
    const NAME: &'static str = "hashbrown";

    fn new() -> Self {
        Self::new()
    }

    fn build(
        &mut self,
        key: Box<[u8]>,
        mut insert: impl FnMut() -> u64,
        mut update: impl FnMut(&mut u64),
    ) {
        use EntryRef::*;
        match self.entry_ref(key.as_ref()) {
            Occupied(mut o) => update(o.get_mut()),
            Vacant(v) => {
                v.insert(insert());
            }
        }
    }

    fn probe(&self, key: &Box<[u8]>) -> Option<u64> {
        self.get(key).copied()
    }

    fn foreach<F: FnMut((&[u8], u64))>(&self, f: F) {
        self.iter().map(|(x, y)| (x.as_ref(), *y)).for_each(f)
    }
}

impl Subject for (DatabendMap<UnsafeBytesRef, u64>, Vec<Box<[u8]>>) {
    const NAME: &'static str = "databend";

    fn new() -> Self {
        (DatabendMap::create(), Vec::with_capacity(8 << 20))
    }

    fn build(
        &mut self,
        key: Box<[u8]>,
        mut insert: impl FnMut() -> u64,
        mut update: impl FnMut(&mut u64),
    ) {
        let mut inserted = false;
        let raw = unsafe { UnsafeBytesRef::new(key.as_ref()) };
        let entity = self.0.insert_key(&raw, &mut inserted);
        if inserted {
            self.1.push(key);
            entity.set_value(insert());
        } else {
            update(entity.get_mut_value());
        }
    }

    fn probe(&self, key: &Box<[u8]>) -> Option<u64> {
        let raw = unsafe { UnsafeBytesRef::new(key.as_ref()) };
        let entity = self.0.find_key(&raw);
        entity.map(|e| *e.get_mut_value())
    }

    fn foreach<F: FnMut((&[u8], u64))>(&self, f: F) {
        self.0
            .iter()
            .map(|e| (e.get_key().as_slice(), *e.get_mut_value()))
            .for_each(f)
    }
}

impl Subject for AdaptiveHashtable<u64> {
    const NAME: &'static str = "saha";

    fn new() -> Self {
        AdaptiveHashtable::<u64>::new()
    }

    fn build(
        &mut self,
        key: Box<[u8]>,
        mut insert: impl FnMut() -> u64,
        mut update: impl FnMut(&mut u64),
    ) {
        match unsafe { self.insert(&key) } {
            Ok(x) => {
                x.write(insert());
            }
            Err(x) => {
                update(x);
            }
        }
    }

    fn probe(&self, key: &Box<[u8]>) -> Option<u64> {
        self.get(key)
    }

    fn foreach<F: FnMut((&[u8], u64))>(&self, mut f: F) {
        self.iter()
            .map(|(k, v)| f((k.as_ref(), v)))
            .for_each(|()| ())
    }
}
