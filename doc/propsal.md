# String Adaptive Hash Table for Databend

邮箱：usamoi@outlook.com

## 背景

StringHashTable是键为字符串等类型的哈希表，服务于Aggregation和Hash Join，需要提供两个操作，`insert`和`lookup`。本项目需要实现为字符串等变长键特殊优化的哈希表，StringHashTable，并合并进databend仓库。

## 方案

### 起点

首先，存在一个简单的哈希表，槽内保存键的哈希和键的指针，以及键值对的值，用线性探测法处理哈希冲突。键用Arena另外保存。值假定是一个`u64`，在指针长64位的系统上恰好能存下一个指针。

```rust
use bumpalo::Bump;

#[repr(align(32))]
struct SlotF {
    s: *const str,
    hash: u64,
    val: u64,
}

struct TableF {
    slots: NonNull<SlotF>,
    arena: Bump,
    cap: usize,
    size: usize,
}
```

### 内联键

数据库需要处理的数据的一个常见特征是平均长度短，根据键长度选择不同的分派方式会很有效[^saha]，于是对非`0xff`结尾（字符串键中不存在字符`0xff`）的两类短键做特别处理。

* 长度在`0..=2`的键。他们可以被完整地映射到65536个槽上单独保存，不需要进入哈希表。

```rust
#[repr(C, align(64))]
struct Slot1 {
    bits: [u64; 4],
    vals: [u64; 256],
}

type Table2 = [Slot1; 256];
```

* 长度在`3..=8`，`9..=16`或是`17..=24`的键。键用`0xff`补齐长度到8，16或24，并与值存放在一起，作为一个Slot。

```rust
#[repr(C, align(16))]
struct Slot8 {
    s: [u8; 8],
    val: u64,
}

#[repr(C, align(8))]
struct Slot16 {
    s: [u8; 16],
    val: u64,
}

#[repr(C, align(32))]
struct Slot24 {
    s: [u8; 24],
    val: u64,
}
```

### 垂直向量化的线性探测

提供垂直向量化的哈希表[^sigmod15]。假定目标机器向量宽度为256，那么可以一次性用4个输入字符串的64位Hash值去寻址，寻址成功时，将该Hash在向量中删除，并用其他需要插入的字符串的Hash作为补充，如此可以更有效地利用SIMD带来的并行性。需要性能测试判断优化是否有效。

具体地，插入实现如下：
1. 接收4个输入的hash和key，并以这些hash作为哈希表中要查找的位置取得index。
2. 以这些index，在哈希表中gather取得key。
3. 比对要插入的key和查找到的key。
4. 查找失败的key的index自增1；查找到空槽的key进行插入，并被selective load替换成新的未被插入的hash和key；查找成功的元素进行更新，并被selective load替换成新的未被插入的hash和key。这里需要处理index相等时的数据冲突。
5. 重复2~4直到输入结束。

在处理`3..=8`，`9..=16`，`17..=24`三种长度的键的哈希表中，我们分别需要1次，2次和3次SIMD比较来确认key是否相同；通用的哈希表在比较hash成功后也需要再比较key，这对效率影响很大。MatrixOne选择以牺牲正确性为代价用192位哈希替代原有长键进行比较[^hash192]，这可能可以作为一个可选功能。

### 垂直向量化的布谷鸟哈希

提供向量化的布谷鸟哈希表[^sigmod15]。布谷鸟哈希允许哈希表拥有更高的负载因子。需要性能测试判断优化是否有效。

具体地，插入实现如下：
1. 接收4个输入的hash_1，hash_2和key，并以这些hash作为哈希表1和哈希表2中要查找的位置取得index_1和index_2。
2. 以这些index_1，在哈希表1中gather取得key_1，取得非空key_1的元素在哈希表2中gather取得key_2，在本次循环时要被重新插入的元素取得非自身来源的key_i。
3. 未取key_2的元素scatter入哈希表1，取key_2的元素scatter入哈希表2，重新插入的元素scatter入哈希表i。
4. 插入成功的元素用输入补充。
5. 重复2~4直到输入结束。

判断插入循环的方法是，每k次迭代检查是否有某位置上的元素自上次检查以来未被新元素替换过。插入循环发生时，哈希表扩容。

### 哈希函数

键的哈希会在进行插入操作之前批量计算，减少向量化代码中访问字符串的内容。键是变长的，但是由于我们对长度在`3..=8`，`9..=16`或是`17..=24`的键实现了分派，这些补`0xff`后长度成为8，16或24的键的哈希依然可以并行哈希。当前对键使用的哈希函数是aHash，它使用AESENC和AESDEC指令进行哈希，可以考虑增加对三种长度的键的特化的循环展开版本，或者使用MurmurHash3作为哈希函数进行向量化。*需要性能测试判断优化是否有效。*

### 可扩展哈希

哈希表被插入的数据的数量通常是难以预估的，需要合适的扩容策略。Databend当前的实现是，最大负载因子是0.5，扩容因子是2，当数据量增大到某一阈值时，重建哈希表为有256个bucket的二层哈希表，每个bucket单独扩容。这里将要实现的一个扩展是可扩展哈希（Extendible hashing），总数据增多时可以调整bucket的数量。需要性能测试判断优化是否有效。

```rust
struct Extendible {
    counter: u8,
    buckets: NonNull<Bucket>,
}

struct Bucket {
    counter: u8,
    table8: TableS<Slot8>,
    table16: TableS<Slot16>,
    table24: TableS<Slot24>,
    table: TableF,
}
```

### 性能测试

测试环境是32 GB内存的云服务器。测试分为两类，一类是与其他哈希表实现的横向对比，一类是实行某种优化后的纵向对比。测试的指标包括峰值内存和耗时。

| 测试集[^dataset] | 描述                                        |
| ---------------- | ------------------------------------------- |
| Paid             | UK Property Price Paid                      |
| Taxi             | New York Taxi Data                          |
| Rcps             | RecipeNLG dataset                           |
| Wiki             | Page view statistics for Wikimedia projects |

| 测试者    | 描述                            |
| --------- | ------------------------------- |
| now       | Databend's hash table           |
| saha      | Clickhouse's hash table         |
| hashbrown | Google's SwissTable (Rust port) |
| cxx       | martinus/robin-hood-hashing     |

| 测试项目    | 描述                                       |
| ----------- | ------------------------------------------ |
| insert_iter | 插入所有键值对，并进行一轮迭代。           |
| insert_find | 插入所有键值对，并按插入顺序进行一轮查找。 |

## 计划

| 日期        | 任务                                             |
| ----------- | ------------------------------------------------ |
| 7.6 - 7.12  | 准备性能测试部分。                               |
| 7.13 - 7.19 | 实现具备内联键的哈希表并进行性能测试部分。       |
| 7.20 - 7.26 | 实现垂直向量化的线性探测部分并进行性能测试。     |
| 7.27 - 8.2  | 实现垂直向量化的布谷鸟哈希部分并进行性能测试。   |
| 8.3 - 8.9   | 实现哈希函数部分和可扩展哈希部分并进行性能测试。 |
| 8.10 - 8.16 | 合并代码回主仓库。                               |

[^saha]: SAHA: A String Adaptive Hash Table for Analytical Databases, https://www.mdpi.com/2076-3417/10/6/1915
[^sigmod15]: Rethinking SIMD Vectorization for In-Memory Databases, http://www.cs.columbia.edu/~orestis/sigmod15.pdf
[^hash192]: 浅谈MatrixOne如何用Go语言设计与实现高性能哈希表，https://www.modb.pro/db/397905
[^dataset]: All datasets can be found in https://clickhouse.com/docs/en/getting-started/example-datasets/.
