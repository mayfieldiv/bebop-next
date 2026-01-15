// Test data generation - C implementation for consistent data across benchmarks
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// Simple PRNG for reproducible test data
static uint64_t prng_state = 0x853c49e6748fea9bULL;

static uint64_t prng_next(void) {
    uint64_t x = prng_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    prng_state = x;
    return x * 0x2545f4914f6cdd1dULL;
}

void prng_seed(uint64_t seed) {
    prng_state = seed;
}

uint64_t prng_u64(void) {
    return prng_next();
}

int32_t prng_i32(void) {
    return (int32_t)(prng_next() & 0xFFFFFFFF);
}

double prng_f64(void) {
    return (double)(prng_next() >> 11) * (1.0 / (1ULL << 53));
}

void prng_bytes(uint8_t* buf, size_t len) {
    for (size_t i = 0; i < len; i++) {
        buf[i] = (uint8_t)(prng_next() & 0xFF);
    }
}
