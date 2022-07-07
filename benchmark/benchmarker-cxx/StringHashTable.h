#pragma once

#include "HashMap.h"
#include "HashTable.h"
#include "Prelude.h"
#include "StringRef.h"

#include <new>
#include <nmmintrin.h>
#include <variant>

using StringKey8 = u64;

struct StringKey16 {
    u64 a;
    u64 b;

    bool operator==(const StringKey16 rhs) const { return a == rhs.a && b == rhs.b; }
};

struct StringKey24 {
    u64 a;
    u64 b;
    u64 c;

    bool operator==(const StringKey24 rhs) const { return a == rhs.a && b == rhs.b && c == rhs.c; }
};

inline StringRef toStringRef(const StringKey8 &n) {
    return {reinterpret_cast<const char *>(&n), 8ul - (__builtin_clzll(n) >> 3)};
}
inline StringRef toStringRef(const StringKey16 &n) {
    return {reinterpret_cast<const char *>(&n), 16ul - (__builtin_clzll(n.b) >> 3)};
}
inline StringRef toStringRef(const StringKey24 &n) {
    return {reinterpret_cast<const char *>(&n), 24ul - (__builtin_clzll(n.c) >> 3)};
}

struct StringHashTableHash {
    size_t inline operator()(StringKey8 key) const {
        size_t res = -1ULL;
        res = _mm_crc32_u64(res, key);
        return res;
    }
    size_t inline operator()(StringKey16 key) const {
        size_t res = -1ULL;
        res = _mm_crc32_u64(res, key.a);
        res = _mm_crc32_u64(res, key.b);
        return res;
    }
    size_t inline operator()(StringKey24 key) const {
        size_t res = -1ULL;
        res = _mm_crc32_u64(res, key.a);
        res = _mm_crc32_u64(res, key.b);
        res = _mm_crc32_u64(res, key.c);
        return res;
    }
    size_t inline operator()(StringRef x) const {
        const char *pos = x.data;
        size_t size = x.size;

        if (size == 0)
            return 0;

        if (size < 8) {
            return hashLessThan8(x.data, x.size);
        }

        const char *end = pos + size;
        size_t res = -1ULL;

        do {
            u64 word = unaligned_read<u64>(pos);
            res = _mm_crc32_u64(res, word);

            pos += 8;
        } while (pos + 8 < end);

        u64 word = unaligned_read<u64>(end - 8); /// I'm not sure if this is normal.
        res = _mm_crc32_u64(res, word);

        return res;
    }
};

template <typename Cell>
struct StringHashTableEmpty {
    using Self = StringHashTableEmpty;

    bool has_zero = false;
    std::aligned_storage_t<sizeof(Cell), alignof(Cell)> zero_value_storage;

public:
    bool hasZero() const { return has_zero; }

    void setHasZero() {
        has_zero = true;
        new (zeroValue()) Cell();
    }

    void setHasZero(const Cell &other) {
        has_zero = true;
        new (zeroValue()) Cell(other);
    }

    void clearHasZero() {
        has_zero = false;
        if (!std::is_trivially_destructible_v<Cell>)
            zeroValue()->~Cell();
    }

    Cell *zeroValue() { return std::launder(reinterpret_cast<Cell *>(&zero_value_storage)); }
    const Cell *zeroValue() const { return std::launder(reinterpret_cast<const Cell *>(&zero_value_storage)); }

    using LookupResult = Cell *;
    using ConstLookupResult = const Cell *;

    void inline emplace(VoidKey _, LookupResult &it, bool &inserted, size_t = 0) {
        if (!hasZero()) {
            setHasZero();
            inserted = true;
        } else
            inserted = false;
        it = zeroValue();
    }

    template <typename Key>
    LookupResult inline find(const Key &, size_t = 0) {
        return hasZero() ? zeroValue() : nullptr;
    }

    template <typename Key>
    ConstLookupResult inline find(const Key &, size_t = 0) const {
        return hasZero() ? zeroValue() : nullptr;
    }

    size_t size() const { return hasZero() ? 1 : 0; }
    bool empty() const { return !hasZero(); }
    size_t getBufferSizeInBytes() const { return sizeof(Cell); }
};

template <size_t initial_size_degree = 8>
struct StringHashTableGrower : public HashTableGrower<initial_size_degree> {
    void increaseSize() { this->size_degree += 1; }
};

template <typename Mapped>
struct StringHashTableLookupResult {
    Mapped *mapped_ptr;
    StringHashTableLookupResult() {}                                              /// NOLINT
    StringHashTableLookupResult(Mapped *mapped_ptr_) : mapped_ptr(mapped_ptr_) {} /// NOLINT
    StringHashTableLookupResult(std::nullptr_t) {}                                /// NOLINT
    const VoidKey getKey() const { return {}; }                                   /// NOLINT
    auto &getMapped() { return *mapped_ptr; }
    auto &operator*() { return *this; }
    auto &operator*() const { return *this; }
    auto *operator->() { return this; }
    auto *operator->() const { return this; }
    explicit operator bool() const { return mapped_ptr; }
    friend bool operator==(const StringHashTableLookupResult &a, const std::nullptr_t &) { return !a.mapped_ptr; }
    friend bool operator==(const std::nullptr_t &, const StringHashTableLookupResult &b) { return !b.mapped_ptr; }
    friend bool operator!=(const StringHashTableLookupResult &a, const std::nullptr_t &) { return a.mapped_ptr; }
    friend bool operator!=(const std::nullptr_t &, const StringHashTableLookupResult &b) { return b.mapped_ptr; }
};

template <typename SubMaps>
class StringHashTable : private boost::noncopyable {
protected:
    static constexpr size_t NUM_MAPS = 5;
    // Map for storing empty string
    using T0 = typename SubMaps::T0;

    // Short strings are stored as numbers
    using T1 = typename SubMaps::T1;
    using T2 = typename SubMaps::T2;
    using T3 = typename SubMaps::T3;

    // Long strings are stored as StringRef along with saved hash
    using Ts = typename SubMaps::Ts;
    using Self = StringHashTable;

    T0 m0;
    T1 m1;
    T2 m2;
    T3 m3;
    Ts ms;

public:
    using Key = StringRef;
    using key_type = Key;
    using mapped_type = typename Ts::mapped_type;
    using value_type = typename Ts::value_type;
    using cell_type = typename Ts::cell_type;

    using LookupResult = StringHashTableLookupResult<typename cell_type::mapped_type>;
    using ConstLookupResult = StringHashTableLookupResult<const typename cell_type::mapped_type>;

    StringHashTable() = default;

    explicit StringHashTable(size_t reserve_for_num_elements)
        : m1{reserve_for_num_elements / 4}, m2{reserve_for_num_elements / 4}, m3{reserve_for_num_elements / 4}, ms{reserve_for_num_elements / 4} {
    }

    StringHashTable(StringHashTable &&rhs) noexcept
        : m1(std::move(rhs.m1)), m2(std::move(rhs.m2)), m3(std::move(rhs.m3)), ms(std::move(rhs.ms)) {
    }

    ~StringHashTable() = default;

    // Dispatch is written in a way that maximizes the performance:
    // 1. Always memcpy 8 times bytes
    // 2. Use switch case extension to generate fast dispatching table
    // 3. Funcs are named callables that can be force_inlined
    //
    // NOTE: It relies on Little Endianness
    //
    // NOTE: It requires padded to 8 bytes keys (IOW you cannot pass
    // std::string here, but you can pass i.e. ColumnString::getDataAt()),
    // since it copies 8 bytes at a time.
    template <typename Self, typename Func>
    static auto inline dispatch(Self &self, const std::string &key, Func &&func) {
        StringHashTableHash hash;
        const StringRef x = StringRef(key);
        const size_t sz = x.size;
        if (sz == 0) {
            return func(self.m0, VoidKey{}, 0);
        }

        if (x.data[sz - 1] == 0) {
            // Strings with trailing zeros are not representable as fixed-size
            // string keys. Put them to the generic table.
            return func(self.ms, std::forward<const std::string &>(key), hash(x));
        }

        const char *p = x.data;
        const char s = (-sz & 7) * 8;
        union {
            StringKey8 k8;
            StringKey16 k16;
            StringKey24 k24;
            u64 n[3];
        };
        switch ((sz - 1) >> 3) {
            case 0: // 1..8 bytes
            {
                // first half page
                if ((reinterpret_cast<uintptr_t>(p) & 2048) == 0) {
                    memcpy(&n[0], p, 8);
                    n[0] &= -1ULL >> s;
                } else {
                    const char *lp = x.data + x.size - 8;
                    memcpy(&n[0], lp, 8);
                    n[0] >>= s;
                }
                return func(self.m1, k8, hash(k8));
            }
            case 1: // 9..16 bytes
            {
                memcpy(&n[0], p, 8);
                const char *lp = x.data + x.size - 8;
                memcpy(&n[1], lp, 8);
                n[1] >>= s;
                return func(self.m2, k16, hash(k16));
            }
            case 2: // 17..24 bytes
            {
                memcpy(&n[0], p, 16);
                const char *lp = x.data + x.size - 8;
                memcpy(&n[2], lp, 8);
                n[2] >>= s;
                return func(self.m3, k24, hash(k24));
            }
            default: // >= 25 bytes
            {
                return func(self.ms, std::forward<const std::string &>(key), hash(x));
            }
        }
    }

    struct EmplaceCallable {
        LookupResult &mapped;
        bool &inserted;

        EmplaceCallable(LookupResult &mapped_, bool &inserted_)
            : mapped(mapped_), inserted(inserted_) {}

        template <typename Map>
        void inline operator()(Map &map, VoidKey key, size_t hash) {
            typename Map::LookupResult result;
            map.emplace(key, result, inserted, hash);
            mapped = &result->getMapped();
        }

        template <typename Map>
        void inline operator()(Map &map, const StringKey8 &key, size_t hash) {
            typename Map::LookupResult result;
            map.emplace(key, result, inserted, hash);
            mapped = &result->getMapped();
        }

        template <typename Map>
        void inline operator()(Map &map, const StringKey16 &key, size_t hash) {
            typename Map::LookupResult result;
            map.emplace(key, result, inserted, hash);
            mapped = &result->getMapped();
        }

        template <typename Map>
        void inline operator()(Map &map, const StringKey24 &key, size_t hash) {
            typename Map::LookupResult result;
            map.emplace(key, result, inserted, hash);
            mapped = &result->getMapped();
        }

        template <typename Map>
        void inline operator()(Map &map, const std::string &key, size_t hash) {
            typename Map::LookupResult result;
            map.emplace(StringRef(key), result, inserted, hash);
            mapped = &result->getMapped();
        }
    };

    void inline emplace(const std::string &key, LookupResult &it, bool &inserted) {
        this->dispatch(*this, key, EmplaceCallable(it, inserted));
    }

    struct FindCallable {
        template <typename Submap>
        auto inline operator()(Submap &map, VoidKey key, size_t hash) {
            auto it = map.find(key, hash);
            if (!it)
                return decltype(&it->getMapped()){};
            else
                return &it->getMapped();
        }

        template <typename Submap>
        auto inline operator()(Submap &map, const StringKey8 &key, size_t hash) {
            auto it = map.find(key, hash);
            if (!it)
                return decltype(&it->getMapped()){};
            else
                return &it->getMapped();
        }

        template <typename Submap>
        auto inline operator()(Submap &map, const StringKey16 &key, size_t hash) {
            auto it = map.find(key, hash);
            if (!it)
                return decltype(&it->getMapped()){};
            else
                return &it->getMapped();
        }

        template <typename Submap>
        auto inline operator()(Submap &map, const StringKey24 &key, size_t hash) {
            auto it = map.find(key, hash);
            if (!it)
                return decltype(&it->getMapped()){};
            else
                return &it->getMapped();
        }

        template <typename Submap>
        auto inline operator()(Submap &map, const std::string &key, size_t hash) {
            auto it = map.find(StringRef(key), hash);
            if (!it)
                return decltype(&it->getMapped()){};
            else
                return &it->getMapped();
        }
    };

    LookupResult inline find(const std::string &x) {
        return dispatch(*this, x, FindCallable{});
    }

    ConstLookupResult inline find(const std::string &x) const {
        return dispatch(*this, x, FindCallable{});
    }

    bool inline has(const std::string &x, size_t = 0) const {
        return dispatch(*this, x, FindCallable{}) != nullptr;
    }

    size_t size() const { return m0.size() + m1.size() + m2.size() + m3.size() + ms.size(); }

    bool empty() const { return m0.empty() && m1.empty() && m2.empty() && m3.empty() && ms.empty(); }

    size_t getBufferSizeInBytes() const {
        return m0.getBufferSizeInBytes() + m1.getBufferSizeInBytes() + m2.getBufferSizeInBytes() + m3.getBufferSizeInBytes() + ms.getBufferSizeInBytes();
    }

    void clearAndShrink() {
        m1.clearHasZero();
        m1.clearAndShrink();
        m2.clearAndShrink();
        m3.clearAndShrink();
        ms.clearAndShrink();
    }
};
