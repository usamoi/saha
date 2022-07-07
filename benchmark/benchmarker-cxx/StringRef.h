#pragma once

#include "Prelude.h"
#include <algorithm>
#include <cstring>
#include <nmmintrin.h>
#include <smmintrin.h>
#include <string>

inline u64 Hash128to64(const u128 &x) {
    const u64 kMul = 0x9ddfea08eb382d69ULL;
    u64 a = (u128_low(x) ^ u128_high(x)) * kMul;
    a ^= (a >> 47);
    u64 b = (u128_high(x) ^ a) * kMul;
    b ^= (b >> 47);
    b *= kMul;
    return b;
}

struct StringRef {
    const char *data = nullptr;
    size_t size = 0;

    /// Non-constexpr due to reinterpret_cast.
    template <typename CharT>
    requires(sizeof(CharT) == 1)
        StringRef(const CharT *data_, size_t size_) : data(reinterpret_cast<const char *>(data_)), size(size_) {
    }

    constexpr StringRef(const char *data_, size_t size_) : data(data_), size(size_) {}

    StringRef(const std::string &s) : data(s.data()), size(s.size()) {} /// NOLINT
    constexpr explicit StringRef(std::string_view s) : data(s.data()), size(s.size()) {}
    constexpr StringRef(const char *data_) : StringRef(std::string_view{data_}) {} /// NOLINT
    constexpr StringRef() = default;

    bool empty() const { return size == 0; }

    std::string toString() const { return std::string(data, size); }

    explicit operator std::string() const { return toString(); }
    std::string_view toView() const { return std::string_view(data, size); }

    constexpr explicit operator std::string_view() const { return std::string_view(data, size); }
};

inline u64 hashLen16(u64 u, u64 v) {
    return Hash128to64(u128(u, v));
}

inline u64 shiftMix(u64 val) {
    return val ^ (val >> 47);
}

inline u64 rotateByAtLeast1(u64 val, int shift) {
    return (val >> shift) | (val << (64 - shift));
}

inline size_t hashLessThan8(const char *data, size_t size) {
    static constexpr u64 k2 = 0x9ae16a3b2f90404fULL;
    static constexpr u64 k3 = 0xc949d7c7509e6557ULL;

    if (size >= 4) {
        u64 a = unaligned_read<u32>(data);
        return hashLen16(size + (a << 3), unaligned_read<u32>(data + size - 4));
    }

    if (size > 0) {
        u8 a = data[0];
        u8 b = data[size >> 1];
        u8 c = data[size - 1];
        u32 y = static_cast<u32>(a) + (static_cast<u32>(b) << 8);
        u32 z = size + (static_cast<u32>(c) << 2);
        return shiftMix(y * k2 ^ z * k3) * k2;
    }

    return k2;
}

inline size_t hashLessThan16(const char *data, size_t size) {
    if (size > 8) {
        u64 a = unaligned_read<u64>(data);
        u64 b = unaligned_read<u64>(data + size - 8);
        return hashLen16(a, rotateByAtLeast1(b + size, size)) ^ b;
    }

    return hashLessThan8(data, size);
}

inline bool compareSSE2(const char *p1, const char *p2) {
    return 0xFFFF == _mm_movemask_epi8(_mm_cmpeq_epi8(
                         _mm_loadu_si128(reinterpret_cast<const __m128i *>(p1)),
                         _mm_loadu_si128(reinterpret_cast<const __m128i *>(p2))));
}

inline bool compareSSE2x4(const char *p1, const char *p2) {
    return 0xFFFF == _mm_movemask_epi8(
                         _mm_and_si128(
                             _mm_and_si128(
                                 _mm_cmpeq_epi8(
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p1)),
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p2))),
                                 _mm_cmpeq_epi8(
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p1) + 1),
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p2) + 1))),
                             _mm_and_si128(
                                 _mm_cmpeq_epi8(
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p1) + 2),
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p2) + 2)),
                                 _mm_cmpeq_epi8(
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p1) + 3),
                                     _mm_loadu_si128(reinterpret_cast<const __m128i *>(p2) + 3)))));
}

inline bool memequalSSE2Wide(const char *p1, const char *p2, size_t size) {
    if (size <= 16) {
        if (size >= 8) {
            /// Chunks of 8..16 bytes.
            return unaligned_read<u64>(p1) == unaligned_read<u64>(p2) && unaligned_read<u64>(p1 + size - 8) == unaligned_read<u64>(p2 + size - 8);
        } else if (size >= 4) {
            /// Chunks of 4..7 bytes.
            return unaligned_read<u32>(p1) == unaligned_read<u32>(p2) && unaligned_read<u32>(p1 + size - 4) == unaligned_read<u32>(p2 + size - 4);
        } else if (size >= 2) {
            /// Chunks of 2..3 bytes.
            return unaligned_read<u16>(p1) == unaligned_read<u16>(p2) && unaligned_read<u16>(p1 + size - 2) == unaligned_read<u16>(p2 + size - 2);
        } else if (size >= 1) {
            /// A single byte.
            return *p1 == *p2;
        }
        return true;
    }

    while (size >= 64) {
        if (compareSSE2x4(p1, p2)) {
            p1 += 64;
            p2 += 64;
            size -= 64;
        } else
            return false;
    }

    switch (size / 16) {
        case 3:
            if (!compareSSE2(p1 + 32, p2 + 32))
                return false;
            [[fallthrough]];
        case 2:
            if (!compareSSE2(p1 + 16, p2 + 16))
                return false;
            [[fallthrough]];
        case 1:
            if (!compareSSE2(p1, p2))
                return false;
    }

    return compareSSE2(p1 + size - 16, p2 + size - 16);
}

inline bool operator==(StringRef lhs, StringRef rhs) {
    if (lhs.size != rhs.size)
        return false;

    if (lhs.size == 0)
        return true;

    return memequalSSE2Wide(lhs.data, rhs.data, lhs.size);
}
