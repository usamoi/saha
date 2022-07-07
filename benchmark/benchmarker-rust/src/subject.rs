use common_hashtable::HashMap as DatabendMap;
use common_hashtable::HashTableEntity;
use hashbrown::hash_map::EntryRef;
use hashbrown::HashMap as HashbrownMap;
use hashtable::hashtable::Hashtable as RefinedHashtable;

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

impl Subject for DatabendMap<Option<Box<[u8]>>, u64> {
    const NAME: &'static str = "databend";

    fn new() -> Self {
        DatabendMap::create()
    }

    fn build(
        &mut self,
        key: Box<[u8]>,
        mut insert: impl FnMut() -> u64,
        mut update: impl FnMut(&mut u64),
    ) {
        let mut inserted = false;
        let entity = self.insert_key(Some(key), &mut inserted);
        if inserted {
            entity.set_value(insert());
        } else {
            update(entity.get_mut_value());
        }
    }

    fn probe(&self, key: &Box<[u8]>) -> Option<u64> {
        let entity = self.find_key(unsafe { std::mem::transmute::<_, &Option<Box<[u8]>>>(key) });
        entity.map(|e| *e.get_mut_value())
    }

    fn foreach<F: FnMut((&[u8], u64))>(&self, f: F) {
        self.iter()
            .map(|e| (e.get_key().as_ref().unwrap().as_ref(), *e.get_mut_value()))
            .for_each(f)
    }
}

impl Subject for RefinedHashtable {
    const NAME: &'static str = "saha";

    fn new() -> Self {
        RefinedHashtable::new()
    }

    fn build(
        &mut self,
        key: Box<[u8]>,
        mut insert: impl FnMut() -> u64,
        mut update: impl FnMut(&mut u64),
    ) {
        match self.insert(&key) {
            Ok(x) => *x = insert(),
            Err(x) => update(x),
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
