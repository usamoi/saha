use std::intrinsics::assume;

#[inline(always)]
pub unsafe fn read_le(data: *const u8, len: usize) -> u64 {
    assume(0 < len && len <= 8);
    let s = 64 - 8 * len as isize;
    if data as usize & 2048 == 0 {
        (data as *const u64).read_unaligned() & (u64::MAX >> s)
    } else {
        (data.offset(len as isize - 8) as *const u64).read_unaligned() >> s
    }
}
