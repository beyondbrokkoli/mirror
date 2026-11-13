
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "sys_sync.h"

void vx_trace_poke(uint8_t slot);
void vx_spin_wait(int* spin_count);

#define STR_KEY_TAG   ((int64_t)INT64_MIN)
#define STR_POOL_BUCKETS 4096

typedef struct StrNode {
    struct StrNode* next;
    char*           data;
    size_t          len;
} StrNode;

static StrNode* pool_buckets[STR_POOL_BUCKETS];

static atomic_flag pool_lock = ATOMIC_FLAG_INIT;

static uint64_t fnv1a_64(const uint8_t* s, size_t len) {
    uint64_t h = 14695981039346656037ULL;
    for (size_t i = 0; i < len; i++) {
        h ^= s[i];
        h *= 1099511628211ULL;
    }
    return h;
}

static int64_t str_key_of(const uint8_t* s, size_t len) {
    return (int64_t)(STR_KEY_TAG | (int64_t)(fnv1a_64(s, len) &
                                             0x3FFFFFFFFFFFFFFFULL));
}

static const uint8_t* pool_intern(const uint8_t* s, size_t len) {
    /* The one-empty law: a zero-length string is the null handle. The
       pool never mints a second empty, so an "" literal's strtab slot
       registers as NULL and handle identity (`==`) needs no
       normalization to agree with the zero lane. */
    if (len == 0) {
        return NULL;
    }
    int spin = 0;
    while (TAS(pool_lock)) {
        vx_spin_wait(&spin);
    }
    uint64_t h = fnv1a_64(s, len);
    StrNode** bucket = &pool_buckets[h % STR_POOL_BUCKETS];
    const uint8_t* out = NULL;
    for (StrNode* n = *bucket; n; n = n->next) {
        if (n->len == len && memcmp(n->data + 8, s, len) == 0) {
            out = (const uint8_t*)n->data + 8;
            break;
        }
    }
    if (out) {
        CLR(pool_lock);
        vx_trace_poke(9);
        return out;
    }
    char* block = malloc(8 + len + 1);
    if (!block) {
        CLR(pool_lock);
        return NULL;
    }
    int64_t key = str_key_of(s, len);
    memcpy(block, &key, 8);
    memcpy(block + 8, s, len);
    block[8 + len] = 0;
    StrNode* node = malloc(sizeof(StrNode));
    if (!node) {
        free(block);
        CLR(pool_lock);
        return NULL;
    }
    node->data = block;
    node->len = len;
    node->next = *bucket;
    *bucket = node;
    CLR(pool_lock);
    vx_trace_poke(8);
    return (const uint8_t*)block + 8;
}

EXPORT const uint8_t* vx_str_intern(const uint8_t* s, size_t len) {
    if (!s) {
        return NULL;
    }
    return pool_intern(s, len);
}

EXPORT void vx_str_register(const uint8_t** strtab, int64_t n) {
    for (int64_t i = 0; i < n; i++) {
        const uint8_t* lit = strtab[i];
        if (lit) {
            strtab[i] = pool_intern(lit, strlen((const char*)lit));
        }
    }
}
