#pragma once

#include "mock_std.h"
#include <algorithm>
#include <cstring>

using usize = unsigned long;
using isize = long;
using u8 = unsigned char;
using i8 = char;
using u16 = unsigned short;
using i16 = short;
using u32 = unsigned int;
using i32 = int;
using u64 = unsigned long;
using i64 = long;
using u128 = std::pair<u64, u64>;
using i128 = std::pair<u64, i64>;

inline u64 u128_low(const u128 &x) { return x.first; }
inline u64 u128_high(const u128 &x) { return x.second; }

template <typename T>
inline T unaligned_read(const void *address) {
    T res{};
    memcpy(&res, address, sizeof(res));
    return res;
}
