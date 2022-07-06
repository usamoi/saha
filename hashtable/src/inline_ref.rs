use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

const LENGTH: usize = 30;

#[derive(Clone, Copy)]
pub struct InlineRefOwned {
    bytes: [u8; LENGTH],
    len: u8,
}

impl AsRef<[u8]> for InlineRefOwned {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[0..self.len as usize]
    }
}

impl Debug for InlineRefOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl PartialEq for InlineRefOwned {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for InlineRefOwned {}

impl PartialOrd for InlineRefOwned {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_ref(), other.as_ref())
    }
}

impl Ord for InlineRefOwned {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.as_ref(), other.as_ref())
    }
}

impl Hash for InlineRefOwned {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}

#[derive(Clone, Copy)]
pub enum InlineRef<'a> {
    Borrowed(&'a [u8]),
    Owned(InlineRefOwned),
}

static_assertions::assert_eq_size!(InlineRef<'static>, [u8; 32]);

impl<'a> InlineRef<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        if bytes.len() <= LENGTH {
            let mut target = [0u8; LENGTH];
            target[0..bytes.len()].copy_from_slice(&bytes);
            Self::Owned(InlineRefOwned {
                len: bytes.len() as u8,
                bytes: target,
            })
        } else {
            Self::Borrowed(bytes)
        }
    }
    pub fn new_owned(bytes: &[u8]) -> Option<Self> {
        if bytes.len() <= LENGTH {
            let mut target = [0u8; LENGTH];
            target[0..bytes.len()].copy_from_slice(&bytes);
            Some(Self::Owned(InlineRefOwned {
                len: bytes.len() as u8,
                bytes: target,
            }))
        } else {
            None
        }
    }
}

impl<'a> AsRef<[u8]> for InlineRef<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            InlineRef::Borrowed(bytes) => bytes,
            InlineRef::Owned(InlineRefOwned { len, bytes }) => &bytes[0..*len as usize],
        }
    }
}

impl<'a> Deref for InlineRef<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a> Debug for InlineRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<'a> PartialEq for InlineRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(self.as_ref(), other.as_ref())
    }
}

impl<'a> Eq for InlineRef<'a> {}

impl<'a> PartialOrd for InlineRef<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self.as_ref(), other.as_ref())
    }
}

impl<'a> Ord for InlineRef<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        Ord::cmp(self.as_ref(), other.as_ref())
    }
}

impl<'a> Hash for InlineRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state);
    }
}
