#include "StringHashMap.h"
#include "json.h"
#include "mock_std.h"
#include "robin_hood.h"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <unistd.h>
#include <vector>
#define let auto
#define fn auto
#define asserta(x) \
    if (!(x))      \
    std::abort()

void *operator new(size_t size) {
    return mock_std::malloc(size);
}

void *operator new[](size_t size) {
    return mock_std::malloc(size);
}

void *operator new(size_t size, std::align_val_t al) {
    return mock_std::aligned_alloc((size_t)al, size);
}

void *operator new[](size_t size, std::align_val_t al) {
    return mock_std::aligned_alloc((size_t)al, size);
}

void *operator new(size_t size, const std::nothrow_t &tag) noexcept {
    return mock_std::malloc(size);
}

void *operator new[](size_t size, const std::nothrow_t &tag) noexcept {
    return mock_std::malloc(size);
}

void *operator new(size_t size, std::align_val_t al, const std::nothrow_t &tag) noexcept {
    return mock_std::aligned_alloc((size_t)al, size);
}

void *operator new[](size_t size, std::align_val_t al, const std::nothrow_t &tag) noexcept {
    return mock_std::aligned_alloc((size_t)al, size);
}

void operator delete(void *ptr) noexcept {
    mock_std::free(ptr);
}

void operator delete[](void *ptr) noexcept {
    mock_std::free(ptr);
}

void operator delete(void *ptr, std::align_val_t al) noexcept {
    mock_std::free(ptr);
}

void operator delete[](void *ptr, std::align_val_t al) noexcept {
    mock_std::free(ptr);
}

void operator delete(void *ptr, std::size_t sz) noexcept {
    mock_std::free(ptr);
}

void operator delete[](void *ptr, std::size_t sz) noexcept {
    mock_std::free(ptr);
}

void operator delete(void *ptr, std::size_t sz, std::align_val_t al) noexcept {
    mock_std::free(ptr);
}

void operator delete[](void *ptr, std::size_t sz, std::align_val_t al) noexcept {
    mock_std::free(ptr);
}

template <typename F>
fn measure_time(F f)->u64 {
    let before = std::chrono::steady_clock::now();
    f();
    let after = std::chrono::steady_clock::now();
    return (after - before).count() / 1000000;
}

template <typename F>
fn read(std::vector<std::string> dataset_files, F f) {
    let strings = std::vector<std::string>();
    let buffer = std::vector<char>(262144);
    let cached = std::string();
    for (let dataset_file : dataset_files) {
        usize length = std::filesystem::file_size(dataset_file);
        std::ifstream file(dataset_file);
        for (u32 i = 0; i < (length + buffer.size() - 1) / buffer.size(); i++) {
            usize n = std::min(length - i * (usize)buffer.size(), (usize)buffer.size());
            file.read(buffer.data(), n);
            asserta(file.good());
            for (u32 j = 0; j < n; j++) {
                char c = buffer[j];
                if (c == ' ' || c == ',' || c == '\n' || c == '\r' || c == '"') {
                    if (cached.length() != 0) {
                        strings.push_back(cached);
                        cached.clear();
                    }
                    if (strings.size() >= 262144) {
                        f(strings);
                        strings.clear();
                    }
                } else {
                    cached.push_back(c);
                }
            }
        }
        if (cached.length() != 0) {
            strings.push_back(cached);
            cached.clear();
        }
        f(strings);
        strings.clear();
    }
}

fn solver_cxx(nlohmann::json manifest) {
    for (let it = manifest.begin(); it != manifest.end(); ++it) {
        nlohmann::json item = *it;
        std::string name = item["name"];
        std::vector<std::string> files = item["files"];
        u64 time_build = 0;
        u64 time_probe = 0;
        u64 time_foreach = 0;
        let subject = robin_hood::unordered_map<std::string, u64>();
        read(files, [&](std::vector<std::string> strings) {
            time_build += measure_time([&] {
                for (std::string string : strings) {
                    if (!subject.count(string)) {
                        subject.insert({std::move(string), 1ull});
                    } else {
                        subject[string] += 1;
                    }
                }
            });
        });
        read(files, [&](std::vector<std::string> strings) {
            time_probe += measure_time([&] {
                for (std::string string : strings) {
                    asserta(subject.count(string));
                }
            });
        });
        u64 count = 0;
        u64 count_distinct = 0;
        for (let[k, v] : subject) {
            count += 1;
            count_distinct += v;
        }
        usize before_dropping = mock_std::usage();
        {
            auto _ = std::move(subject);
        }
        usize after_dropping = mock_std::usage();
        usize memory = before_dropping - after_dropping;
        std::cout << "cxx," << name << ","
                  << time_build << ","
                  << time_probe << ","
                  << time_foreach << ","
                  << memory << ","
                  << count << ","
                  << count_distinct << "\n";
    }
}

fn solver_clickhouse(nlohmann::json manifest) {
    for (let it = manifest.begin(); it != manifest.end(); ++it) {
        nlohmann::json item = *it;
        std::string name = item["name"];
        std::vector<std::string> files = item["files"];
        u64 time_build = 0;
        u64 time_probe = 0;
        u64 time_foreach = 0;
        let subject = StringHashMap<u64>();
        let arena = std::vector<std::string>();
        read(files, [&](std::vector<std::string> strings) {
            time_build += measure_time([&] {
                for (std::string string : std::move(strings)) {
                    StringHashMap<u64>::LookupResult it;
                    bool inserted = false;
                    subject.emplace(string, it, inserted);
                    if (inserted) {
                        it->getMapped() = 1;
                        if (string.length() >= 25) {
                            arena.push_back(std::move(string));
                        }
                    } else {
                        it->getMapped() += 1;
                    }
                }
            });
        });
        read(files, [&](std::vector<std::string> strings) {
            time_probe += measure_time([&] {
                for (std::string string : strings) {
                    asserta(subject.find(string) != nullptr);
                }
            });
        });
        u64 count = 0;
        u64 count_distinct = 0;
        subject.forEachValue([&](StringRef k, u64 v) {
            count += 1;
            count_distinct += v;
        });
        usize before_dropping = mock_std::usage();
        {
            auto _subject = std::move(subject);
            auto _arena = std::move(arena);
        }
        usize after_dropping = mock_std::usage();
        usize memory = before_dropping - after_dropping;
        std::cout << "cxx," << name << ","
                  << time_build << ","
                  << time_probe << ","
                  << time_foreach << ","
                  << memory << ","
                  << count << ","
                  << count_distinct << "\n";
    }
}

int main(int argc, char *argv[]) {
    if (argc == 3 && (strcmp(argv[1], "-p") || strcmp(argv[1], "--path"))) {
        chdir(argv[2]);
    } else if (argc != 1) {
        std::cout << "fatal error: unknown argument";
        return 1;
    }
    std::ifstream manifest_file("manifest.json");
    std::stringstream manifest_buffer;
    manifest_buffer << manifest_file.rdbuf();
    nlohmann::json manifest = nlohmann::json::parse(manifest_buffer);
    std::cout << "subject,dataset,time_build,time_probe,time_foreach,memory,count,count_distinct\n";
    solver_clickhouse(manifest);
    solver_cxx(manifest);
    return 0;
}
