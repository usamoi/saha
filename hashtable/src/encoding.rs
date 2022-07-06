#[repr(C, align(64))]
struct Group {
    bits: u64,
    slots: [u64; 64],
}

pub struct EncodingHashtable {
    groups: [Group; 1024],
    len: usize,
}

impl EncodingHashtable {
    pub fn new() -> Self {
        const EMPTY_GROUP: Group = Group {
            bits: 0,
            slots: [0; 64],
        };
        Self {
            groups: [EMPTY_GROUP; 1024],
            len: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, key: [u8; 2]) -> Option<u64> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.groups[x].bits & (1 << y)) != 0;
        if z {
            Some(self.groups[x].slots[y as usize])
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, key: [u8; 2]) -> Option<&mut u64> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.groups[x].bits & (1 << y)) != 0;
        if z {
            Some(&mut self.groups[x].slots[y as usize])
        } else {
            None
        }
    }
    pub fn insert(&mut self, key: [u8; 2]) -> Result<&mut u64, &mut u64> {
        let x = ((key[0] as usize) << 2) | (key[1] as usize >> 6);
        let y = key[1] & 63;
        let z = (self.groups[x].bits & (1 << y)) != 0;
        if z {
            Err(&mut self.groups[x].slots[y as usize])
        } else {
            self.len += 1;
            self.groups[x].bits |= 1 << y;
            Ok(&mut self.groups[x].slots[y as usize])
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = ([u8; 2], u64)> + '_ {
        self.groups
            .iter()
            .enumerate()
            .map(|(x, group)| {
                let mut bits = group.bits;
                std::iter::from_fn(move || {
                    let y = bits.checked_log2()?;
                    bits -= 1 << y;
                    let i = (x >> 2) as u8;
                    let j = ((x & 3) << 6) as u8 + y as u8;
                    Some(([i, j], group.slots[y as usize]))
                })
            })
            .flatten()
    }
}
