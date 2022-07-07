#pragma once

#include "HashTable.h"

struct NoInitTag {
};

/// A pair that does not initialize the elements, if not needed.
template <typename First, typename Second>
struct PairNoInit {
    First first;
    Second second;

    PairNoInit() {} /// NOLINT

    template <typename FirstValue>
    PairNoInit(FirstValue &&first_, NoInitTag)
        : first(std::forward<FirstValue>(first_)) {
    }

    template <typename FirstValue, typename SecondValue>
    PairNoInit(FirstValue &&first_, SecondValue &&second_)
        : first(std::forward<FirstValue>(first_)), second(std::forward<SecondValue>(second_)) {
    }
};

template <typename First, typename Second>
PairNoInit<std::decay_t<First>, std::decay_t<Second>> makePairNoInit(First &&first, Second &&second) {
    return PairNoInit<std::decay_t<First>, std::decay_t<Second>>(std::forward<First>(first), std::forward<Second>(second));
}

template <typename Key, typename Value, typename Hash, typename TState = HashTableNoState>
struct HashMapCell {
    using Mapped = Value;
    using State = TState;

    using value_type = PairNoInit<Key, Mapped>;
    using mapped_type = Mapped;
    using key_type = Key;

    value_type value;

    HashMapCell() = default;
    HashMapCell(const Key &key_, const State &) : value(key_, NoInitTag()) {}
    HashMapCell(const value_type &value_, const State &) : value(value_) {}

    /// Get the key (externally).
    const Key &getKey() const { return value.first; }
    Mapped &getMapped() { return value.second; }
    const Mapped &getMapped() const { return value.second; }
    const value_type &getValue() const { return value; }

    /// Get the key (internally).
    static const Key &getKey(const value_type &value) { return value.first; }

    bool keyEquals(const Key &key_) const { return bitEquals(value.first, key_); }
    bool keyEquals(const Key &key_, size_t /*hash_*/) const { return bitEquals(value.first, key_); }
    bool keyEquals(const Key &key_, size_t /*hash_*/, const State & /*state*/) const { return bitEquals(value.first, key_); }

    void setHash(size_t /*hash_value*/) {}
    size_t getHash(const Hash &hash) const { return hash(value.first); }

    bool isZero(const State &state) const { return isZero(value.first, state); }
    static bool isZero(const Key &key, const State & /*state*/) { return ZeroTraits::check(key); }

    /// Set the key value to zero.
    void setZero() { ZeroTraits::set(value.first); }

    /// Do I need to store the zero key separately (that is, can a zero key be inserted into the hash table).
    static constexpr bool need_zero_value_storage = true;

    void setMapped(const value_type &value_) { value.second = value_.second; }

    static bool constexpr need_to_notify_cell_during_move = false;

    static void move(HashMapCell * /* old_location */, HashMapCell * /* new_location */) {}

    template <size_t I>
    auto &get() & {
        if constexpr (I == 0)
            return value.first;
        else if constexpr (I == 1)
            return value.second;
    }

    template <size_t I>
    auto const &get() const & {
        if constexpr (I == 0)
            return value.first;
        else if constexpr (I == 1)
            return value.second;
    }

    template <size_t I>
    auto &&get() && {
        if constexpr (I == 0)
            return std::move(value.first);
        else if constexpr (I == 1)
            return std::move(value.second);
    }
};

namespace std {

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_size<HashMapCell<Key, Value, Hash, TState>> : std::integral_constant<size_t, 2> {};

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_element<0, HashMapCell<Key, Value, Hash, TState>> { using type = Key; };

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_element<1, HashMapCell<Key, Value, Hash, TState>> { using type = Value; };
}

template <typename Key, typename Value, typename Hash, typename TState = HashTableNoState>
struct HashMapCellWithSavedHash : public HashMapCell<Key, Value, Hash, TState> {
    using Base = HashMapCell<Key, Value, Hash, TState>;

    size_t saved_hash;

    using Base::Base;

    bool keyEquals(const Key &key_) const { return bitEquals(this->value.first, key_); }
    bool keyEquals(const Key &key_, size_t hash_) const { return saved_hash == hash_ && bitEquals(this->value.first, key_); }
    bool keyEquals(const Key &key_, size_t hash_, const typename Base::State &) const { return keyEquals(key_, hash_); }

    void setHash(size_t hash_value) { saved_hash = hash_value; }
    size_t getHash(const Hash & /*hash_function*/) const { return saved_hash; }
};

template <
    typename Key,
    typename Cell,
    typename Hash,
    typename Grower = HashTableGrower<>>
class HashMapTable : public HashTable<Key, Cell, Hash, Grower> {
public:
    using Self = HashMapTable;
    using Base = HashTable<Key, Cell, Hash, Grower>;
    using LookupResult = typename Base::LookupResult;

    using Base::Base;

    /// Merge every cell's value of current map into the destination map via emplace.
    ///  Func should have signature void(Mapped & dst, Mapped & src, bool emplaced).
    ///  Each filled cell in current map will invoke func once. If that map doesn't
    ///  have a key equals to the given cell, a new cell gets emplaced into that map,
    ///  and func is invoked with the third argument emplaced set to true. Otherwise
    ///  emplaced is set to false.
    template <typename Func>
    void inline mergeToViaEmplace(Self &that, Func &&func) {
        for (auto it = this->begin(), end = this->end(); it != end; ++it) {
            typename Self::LookupResult res_it;
            bool inserted;
            that.emplace(Cell::getKey(it->getValue()), res_it, inserted, it.getHash());
            func(res_it->getMapped(), it->getMapped(), inserted);
        }
    }

    /// Merge every cell's value of current map into the destination map via find.
    ///  Func should have signature void(Mapped & dst, Mapped & src, bool exist).
    ///  Each filled cell in current map will invoke func once. If that map doesn't
    ///  have a key equals to the given cell, func is invoked with the third argument
    ///  exist set to false. Otherwise exist is set to true.
    template <typename Func>
    void inline mergeToViaFind(Self &that, Func &&func) {
        for (auto it = this->begin(), end = this->end(); it != end; ++it) {
            auto res_it = that.find(Cell::getKey(it->getValue()), it.getHash());
            if (!res_it)
                func(it->getMapped(), it->getMapped(), false);
            else
                func(res_it->getMapped(), it->getMapped(), true);
        }
    }

    /// Call func(const Key &, Mapped &) for each hash map element.
    template <typename Func>
    void forEachValue(Func &&func) {
        for (auto &v : *this)
            func(v.getKey(), v.getMapped());
    }

    /// Call func(Mapped &) for each hash map element.
    template <typename Func>
    void forEachMapped(Func &&func) {
        for (auto &v : *this)
            func(v.getMapped());
    }

    typename Cell::Mapped &operator[](const Key &x) {
        LookupResult it;
        bool inserted;
        this->emplace(x, it, inserted);

        /** It may seem that initialization is not necessary for POD-types (or __has_trivial_constructor),
         *  since the hash table memory is initially initialized with zeros.
         * But, in fact, an empty cell may not be initialized with zeros in the following cases:
         * - ZeroValueStorage (it only zeros the key);
         * - after resizing and moving a part of the cells to the new half of the hash table, the old cells also have only the key to zero.
         *
         * On performance, there is almost always no difference, due to the fact that it->second is usually assigned immediately
         *  after calling `operator[]`, and since `operator[]` is inlined, the compiler removes unnecessary initialization.
         *
         * Sometimes due to initialization, the performance even grows. This occurs in code like `++map[key]`.
         * When we do the initialization, for new cells, it's enough to make `store 1` right away.
         * And if we did not initialize, then even though there was zero in the cell,
         *  the compiler can not guess about this, and generates the `load`, `increment`, `store` code.
         */
        if (inserted)
            new (&it->getMapped()) typename Cell::Mapped();

        return it->getMapped();
    }
};

namespace std {

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_size<HashMapCellWithSavedHash<Key, Value, Hash, TState>> : std::integral_constant<size_t, 2> {};

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_element<0, HashMapCellWithSavedHash<Key, Value, Hash, TState>> { using type = Key; };

    template <typename Key, typename Value, typename Hash, typename TState>
    struct tuple_element<1, HashMapCellWithSavedHash<Key, Value, Hash, TState>> { using type = Value; };
}

template <
    typename Key,
    typename Mapped,
    typename Hash,
    typename Grower = HashTableGrower<>>
using HashMap = HashMapTable<Key, HashMapCell<Key, Mapped, Hash>, Hash, Grower>;

template <
    typename Key,
    typename Mapped,
    typename Hash,
    typename Grower = HashTableGrower<>>
using HashMapWithSavedHash = HashMapTable<Key, HashMapCellWithSavedHash<Key, Mapped, Hash>, Hash, Grower>;
