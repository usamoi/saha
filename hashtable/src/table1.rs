use crate::traits::Value;
use std::mem::MaybeUninit;

static ALLKEYS: [[[u8; 2]; 256]; 256] = {
    let mut ans = [[[0u8; 2]; 256]; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut j = 0usize;
        while j < 256 {
            ans[i][j] = [i as u8, j as u8];
            j += 1;
        }
        i += 1;
    }
    ans
};

struct Table1Inner<V: Value> {
    data: [[MaybeUninit<V>; 64]; 1024],
    bits: [u64; 1024],
}

pub struct Table1<V: Value> {
    inner: Box<Table1Inner<V>>,
    len: usize,
}

impl<V: Value> Table1<V> {
    pub fn new() -> Self {
        Self {
            inner: unsafe { Box::<Table1Inner<V>>::new_zeroed().assume_init() },
            len: 0,
        }
    }
    pub fn capacity(&self) -> usize {
        65536
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, key: [u8; 2]) -> Option<V::Ref<'_>> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.inner.bits[x] & (1 << y)) != 0;
        if z {
            Some(unsafe { self.inner.data[x][y as usize].assume_init_ref().as_ref() })
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, key: [u8; 2]) -> Option<&mut V> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.inner.bits[x] & (1 << y)) != 0;
        if z {
            Some(unsafe { self.inner.data[x][y as usize].assume_init_mut() })
        } else {
            None
        }
    }
    /// # Safety
    ///
    /// The resulted `MaybeUninit` should be initialized immedidately.
    pub fn insert(&mut self, key: [u8; 2]) -> Result<&mut MaybeUninit<V>, &mut V> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.inner.bits[x] & (1 << y)) != 0;
        if z {
            Err(unsafe { self.inner.data[x][y as usize].assume_init_mut() })
        } else {
            self.len += 1;
            self.inner.bits[x] |= 1 << y;
            Ok(&mut self.inner.data[x][y as usize])
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = (&[u8; 2], V::Ref<'_>)> + '_ {
        self.inner
            .data
            .iter()
            .enumerate()
            .map(|(x, group)| {
                let mut bits = self.inner.bits[x];
                std::iter::from_fn(move || {
                    let y = bits.trailing_zeros();
                    if y == u64::BITS {
                        return None;
                    }
                    bits ^= 1 << y;
                    let i = (x >> 2) as u8;
                    let j = ((x & 3) << 6) as u8 | y as u8;
                    let k = &ALLKEYS[i as usize][j as usize];
                    let v = unsafe { group[y as usize].assume_init_ref() }.as_ref();
                    Some((k, v))
                })
            })
            .flatten()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&[u8; 2], &mut V)> + '_ {
        self.inner
            .data
            .iter_mut()
            .enumerate()
            .map(|(x, group)| {
                let mut bits = self.inner.bits[x];
                std::iter::from_fn(move || {
                    let y = bits.trailing_zeros();
                    if y == u64::BITS {
                        return None;
                    }
                    bits ^= 1 << y;
                    let i = (x >> 2) as u8;
                    let j = ((x & 3) << 6) as u8 | y as u8;
                    let k = &ALLKEYS[i as usize][j as usize];
                    let v = unsafe { &mut *(group[y as usize].assume_init_mut() as *mut V) };
                    Some((k, v))
                })
            })
            .flatten()
    }
}

impl<V: Value> Drop for Table1<V> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<V>() {
            self.iter_mut().for_each(|(_, v)| unsafe {
                std::ptr::drop_in_place(v);
            });
        }
    }
}