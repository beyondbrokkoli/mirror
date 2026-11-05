
#include <dlfcn.h>
#include <stdio.h>

#include "sys_sync.h"
#include "alm_async.h"
#include "state_types.c"

static _Thread_local int32_t t_coro_lane = -1;

static void* coro_thread_fn(void* argp) {
    CoroSlot* co = (CoroSlot*)argp;
    int32_t lane = (int32_t)(co - g_coros.slots);
    t_coro_lane = lane;

    void* (*x_exec)(void*) = (void* (*)(void*))dlsym(co->lib, "alm_exec");
    void (*x_free)(void*) = (void (*)(void*))dlsym(co->lib, "alm_tbl_free");
    int32_t (*x_exec_id)(void) =
        (int32_t (*)(void))dlsym(co->lib, "alm_rt_exec_id");
    int32_t (*x_kind)(void) = (int32_t (*)(void))dlsym(co->lib, "alm_arg_kind");
    void* (*x_new)(uint64_t, uint8_t) =
        (void* (*)(uint64_t, uint8_t))dlsym(co->lib, "alm_tbl_new");
    void (*x_set)(void*, int64_t, const void*) =
        (void (*)(void*, int64_t, const void*))dlsym(co->lib, "alm_tbl_set");

    if (x_exec == NULL || x_free == NULL) {
        fprintf(stderr,
                "alm coro: module lacks alm_exec/alm_tbl_free — idle exit\n");
    } else {
        void* args = NULL;
        if (x_kind != NULL && x_new != NULL && x_set != NULL && x_kind() >= 0) {
            int32_t kind = x_kind();
            uint64_t esize = (kind == 2) ? 1u : (kind == 4) ? 16u : 8u;
            args = x_new(esize, (uint8_t)((kind & 0x07) << 1));
            if (args != NULL) {
                uint64_t ins[3] = {
                    L_R(co->in_a),
                    L_R(co->in_b),
                    L_R(co->in_c),
                };
                if (kind == 0 || kind == 1) {
                    for (int i = 0; i < 3; i++) x_set(args, i, &ins[i]);
                } else if (kind == 2) {
                    for (int i = 0; i < 3; i++) {
                        uint8_t b = ins[i] ? 1 : 0;
                        x_set(args, i, &b);
                    }
                } else if (kind == 4) {
                    for (int i = 0; i < 3; i++) {
                        unsigned __int128 cell = (unsigned __int128)ins[i];
                        x_set(args, i, &cell);
                    }
                }
            }
        }
        void* result = x_exec(args);
        if (result != NULL && result != args) x_free(result);
        if (args != NULL) x_free(args);
        if (x_exec_id != NULL) {
            int32_t id = x_exec_id();
            if (id >= 0) vx_exec_state_set(id, VX_EXEC_FINISHED);
        }
    }
    S(co->thread_done, 1);
    return THREAD_RETURN_VAL;
}

static CoroSlot* coro_slot(int32_t id) {
    if (id < 0 || id >= VX_CORO_MAX) return NULL;
    return &g_coros.slots[id];
}

static void coro_retire(CoroSlot* co, uint32_t from) {
    uint32_t expected = from;
    if (!CXS(co->state, expected, VX_CORO_DEAD)) return;
    expected = VX_CORO_DEAD;
    CXS(co->state, expected, VX_CORO_EMPTY);
}

EXPORT int32_t vx_coro_create(void* lib) {
    vx_async_ready();
    for (int i = 0; i < VX_CORO_MAX; i++) {
        _Atomic uint32_t* st = &g_coros.slots[i].state;
        uint32_t expected = VX_CORO_EMPTY;
        if (CXS(*st, expected, VX_CORO_CREATED)) {
            g_coros.slots[i].lib = lib;
            atomic_init(&g_coros.slots[i].thread_done, 0);
            atomic_init(&g_coros.slots[i].joined, 0);
            atomic_init(&g_coros.slots[i].abort, 0);
            atomic_init(&g_coros.slots[i].in_a, 0);
            atomic_init(&g_coros.slots[i].in_b, 0);
            atomic_init(&g_coros.slots[i].in_c, 0);
            atomic_init(&g_coros.slots[i].yield_seq, 0);
            g_coros.slots[i].held_idx = -1;
            S(g_ring.ready_idx[i], -1);
            S(g_ring.local_read[i], -1);
            FA(g_ring.locked_mask, ~((0xFu << (i * 4))));
            return (int32_t)i;
        }
    }
    return -1;
}

EXPORT void vx_coro_send(int32_t id, uint64_t a, uint64_t b, uint64_t c) {
    CoroSlot* co = coro_slot(id);
    if (co == NULL) return;
    S_R(co->in_a, a);
    S_R(co->in_b, b);
    S_R(co->in_c, c);
}

EXPORT int32_t vx_coro_start(int32_t id) {
    CoroSlot* co = coro_slot(id);
    if (co == NULL) return -1;
    uint32_t expected = VX_CORO_CREATED;
    if (CXS(co->state, expected, VX_CORO_RUNNING)) {
        co->thread = vx_thread_start(coro_thread_fn, co);
        if (co->thread == 0) {
            fprintf(stderr,
                    "alm coro: lane %d — pthread_create failed; lane retired\n",
                    id);
            S(co->thread_done, 1);
            coro_retire(co, VX_CORO_RUNNING);
            return -1;
        }
        return id;
    }
    return (L(co->state) == VX_CORO_RUNNING || L(co->state) == VX_CORO_DEAD)
               ? id
               : -1;
}

EXPORT int64_t vx_coro_pull(int32_t id, int32_t* done) {
    CoroSlot* co = coro_slot(id);
    if (co == NULL) {
        if (done != NULL) *done = 1;
        return 0;
    }
    int spin = 0;
    for (;;) {
        int idx = vx_stream_next(id);
        if (idx >= 0) {
            if (co->held_idx != -1 && co->held_idx != idx) {
                vx_stream_release(id, co->held_idx);
            }
            co->held_idx = idx;
            RingPacket* p = vx_stream_packet(idx);
            int64_t value = (int64_t)p->a;
            if (done != NULL) *done = 0;
            return value;
        }
        if (L(co->thread_done) == 1) {
            if (co->held_idx != -1) {
                vx_stream_release(id, co->held_idx);
                co->held_idx = -1;
            }
            int32_t expected = 0;
            if (CXS(co->joined, expected, 1)) {
                vx_thread_join(co->thread);
            }
            coro_retire(co, VX_CORO_RUNNING);
            if (done != NULL) *done = 1;
            return 0;
        }
        if (L_R(g_coros.shutdown)) {
            if (done != NULL) *done = 1;
            return 0;
        }
        vx_spin_wait(&spin);
    }
}

EXPORT int64_t vx_coro_yield(uint64_t value) {
    if (value == UINT64_C(0x8000000000000000)) return -2;
    int32_t lane = t_coro_lane;
    if (lane < 0) return -1;
    CoroSlot* co = &g_coros.slots[lane];
    int spin = 0;
    for (;;) {
        if (L_R(g_coros.shutdown) || L_R(co->abort)) return -1;
        int idx = vx_stream_acquire(lane);
        if (idx >= 0) {
            RingPacket* p = vx_stream_packet(idx);
            uint64_t seq = atomic_fetch_add_explicit(&co->yield_seq, 1,
                                                     memory_order_relaxed) + 1;
            p->seq = seq;
            p->a = value;
            p->b = 0;
            p->c = 0;
            p->crc = seq ^ value;
            vx_stream_commit(lane, idx);
            return 0;
        }
        vx_spin_wait(&spin);
    }
}

EXPORT uint32_t vx_coro_status(int32_t id) {
    CoroSlot* co = coro_slot(id);
    if (co == NULL) return VX_CORO_DEAD;
    uint32_t st = L(co->state);
    if (st == VX_CORO_RUNNING && L(co->thread_done) == 1) {
        return VX_CORO_DEAD;
    }
    return st;
}

EXPORT void vx_coro_join(int32_t id) {
    CoroSlot* co = coro_slot(id);
    if (co == NULL) return;
    if (L(co->state) == VX_CORO_CREATED) {
        coro_retire(co, VX_CORO_CREATED);
        return;
    }
    if (L(co->state) != VX_CORO_RUNNING) return;
    S(co->abort, 1);
    int spin = 0;
    while (L(co->thread_done) == 0) vx_spin_wait(&spin);
    int32_t expected = 0;
    if (CXS(co->joined, expected, 1)) {
        vx_thread_join(co->thread);
    }
    coro_retire(co, VX_CORO_RUNNING);
}

EXPORT void vx_coro_quiesce(void) {
    S(g_coros.shutdown, 1);
    for (int i = 0; i < VX_CORO_MAX; i++) {
        CoroSlot* co = &g_coros.slots[i];
        if (L(co->state) != VX_CORO_RUNNING) continue;
        int spin = 0;
        while (L(co->thread_done) == 0) vx_spin_wait(&spin);
        int32_t expected = 0;
        if (CXS(co->joined, expected, 1)) {
            vx_thread_join(co->thread);
        }
        coro_retire(co, VX_CORO_RUNNING);
    }
}
