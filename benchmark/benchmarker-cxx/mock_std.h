#pragma once
#include <cstdlib>
#include <malloc.h>

namespace mock_std {
    size_t measure_memory = 0;
    void *calloc(std::size_t num, std::size_t size) {
        auto ptr = std::calloc(num, size);
        measure_memory += malloc_usable_size(ptr);
        return ptr;
    }
    void *malloc(std::size_t size) {
        auto ptr = std::malloc(size);
        measure_memory += malloc_usable_size(ptr);
        return ptr;
    }
    void *realloc(void *ptr, std::size_t new_size) {
        auto ptx = std::realloc(ptr, new_size);
        measure_memory += malloc_usable_size(ptx);
        return ptx;
    }
    void *aligned_alloc(std::size_t alignment, std::size_t size) {
        auto ptr = std::aligned_alloc(alignment, size);
        measure_memory += malloc_usable_size(ptr);
        return ptr;
    }
    void free(void *ptr) {
        measure_memory -= malloc_usable_size(ptr);
        std::free(ptr);
    }
    size_t usage() {
        return measure_memory;
    }
}
