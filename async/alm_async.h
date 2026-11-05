#ifndef ALM_ASYNC_H
#define ALM_ASYNC_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VX_MAX_WINDOWS 4
#define VX_RING_SIZE   16


#define VX_EXEC_MAX          8
#define VX_EXEC_NAME_MAX     56
#define VX_TRACE_SLOTS       256   /* must match the plate SSOT (1 byte/slot) */
#define VX_TRACE_RING_CAP    8192
#define VX_TRACE_EVENT_NONE  0xFFFFFFFFu

#define VX_EXEC_EMPTY    0u
#define VX_EXEC_BOOTING  1u
#define VX_EXEC_RUNNING  2u
#define VX_EXEC_FINISHED 3u
#define VX_EXEC_FAILED   4u

#define VX_EXEC_F_EXE 0x1u   /* --exe alm binary (alm_exec_main world) */
#define VX_EXEC_F_SO  0x2u   /* alm shared module (driven over the C ABI) */

#define VX_TRACE_SLOT_LEAK 90

typedef struct {
    uint64_t seq;
    uint64_t a;
    uint64_t b;
    uint64_t c;
    uint64_t crc;
} RingPacket;

void vx_init_mailbox(void);

int  vx_stream_acquire(int win_id);
void vx_stream_commit(int win_id, int idx);
void vx_stream_release(int win_id, int idx);
int  vx_stream_next(int win_id);
void vx_stream_dump(int win_id);
RingPacket* vx_stream_packet(int idx);

void vx_spin_wait(int* spin_count);

uint8_t vx_key_get(int win_id, int key);
void    vx_key_set(int win_id, int key, uint8_t val);
float   vx_mouse_dx_get(int win_id);
void    vx_mouse_delta_set(int win_id, float dx, float dy);
int     vx_running_get(void);
void    vx_running_set(int running);

void        vx_async_ready(void);
int32_t     vx_exec_register(const char* name, uint32_t flags);
const char* vx_exec_name(int32_t id);
uint32_t    vx_exec_state_get(int32_t id);
void        vx_exec_state_set(int32_t id, uint32_t state);
uint32_t    vx_exec_flags_get(int32_t id);
void        vx_exec_census_set(int32_t id, int64_t live, int64_t total);
int32_t     vx_exec_count(void);
void        vx_exec_dump(void);

void    vx_trace_poke(uint8_t slot);
void    vx_trace_poke_exec(int32_t exec_id, uint8_t slot);
uint8_t vx_trace_fired(uint8_t slot);

uint64_t vx_trace_event_count(void);
uint32_t vx_trace_event_get(uint64_t seq);


#define VX_CORO_MAX VX_MAX_WINDOWS   /* one ring lane per coroutine */

#define VX_CORO_EMPTY   0u
#define VX_CORO_CREATED 1u   /* context ready, thread not started */
#define VX_CORO_RUNNING 2u   /* pthread live (or finished, pre-join) */
#define VX_CORO_DEAD    3u   /* joined, slot recyclable */

vmath_thread_t vx_thread_start(void* (*func)(void*), void* arg);
void           vx_thread_join(vmath_thread_t thread);

int32_t  vx_coro_create(void* lib);
void     vx_coro_send(int32_t id, uint64_t a, uint64_t b, uint64_t c);
int32_t  vx_coro_start(int32_t id);
int64_t  vx_coro_pull(int32_t id, int32_t* done);
int64_t  vx_coro_yield(uint64_t value);
uint32_t vx_coro_status(int32_t id);
void     vx_coro_join(int32_t id);
void     vx_coro_quiesce(void);

const uint8_t* vx_str_intern(const uint8_t* s, size_t len);
void           vx_str_register(const uint8_t** strtab, int64_t n);

#ifdef __cplusplus
}
#endif

#endif
