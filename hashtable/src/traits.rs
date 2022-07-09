use std::mem::MaybeUninit;

pub unsafe trait Key: Sized + Eq {
    fn equals_zero(this: &Self) -> bool;

    fn is_zero(this: &MaybeUninit<Self>) -> bool;

    fn hash(&self) -> u64;
}

pub trait Value: Sized {
    type Ref<'a>
    where
        Self: 'a;

    fn as_ref(&self) -> Self::Ref<'_>;
}

macro_rules! impl_value_for_primitive {
    ($t: ty) => {
        impl Value for $t {
            type Ref<'a> = Self;

            #[inline(always)]
            fn as_ref(&self) -> Self {
                *self
            }
        }
    };
}

impl_value_for_primitive!(u8);
impl_value_for_primitive!(i8);
impl_value_for_primitive!(u32);
impl_value_for_primitive!(i32);
impl_value_for_primitive!(u64);
impl_value_for_primitive!(i64);
impl_value_for_primitive!(f32);
impl_value_for_primitive!(f64);
