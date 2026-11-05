
#include <stdint.h>
#include <stdio.h>
#include <stdatomic.h>
#include <stdalign.h>
#include <stddef.h>
#include "sys_sync.h"
#include "alm_async.h"

#define OS_CMD_IDLE          0
#define OS_CMD_BOOT_WINDOW   1
#define OS_CMD_KILL_WINDOW   2

#define RND_CMD_IDLE         0
#define RND_CMD_REBUILD_WSI  1
#define RND_CMD_HALT         2
#define RND_CMD_INJECT_TENANT 3

typedef struct {
    _Atomic(void*)  vk_instance;
    _Atomic(void*)  vk_surface;
    _Atomic int     glfw_cmd;
    _Atomic int     render_cmd;
    _Atomic int     glfw_arg_w;
    _Atomic int     glfw_arg_h;

    _Atomic uint8_t keys[512];

    _Atomic float   mouse_dx;
    _Atomic float   mouse_dy;
    _Atomic float   mouse_x;
    _Atomic float   mouse_y;
    _Atomic int     mouse_captured;
    _Atomic int     window_resized;
    _Atomic int     win_w;
    _Atomic int     win_h;
    _Atomic float   click_x;
    _Atomic float   click_y;
    _Atomic uint8_t mouse_btns[8];

    uint8_t         _pad[48];
} TenantMailbox;

_Static_assert(sizeof(TenantMailbox) == 640, "TenantMailbox must prevent false sharing");

typedef struct {
    alignas(64) _Atomic int ready_index;
    _Atomic int is_running;
    _Atomic int lua_finished;
    _Atomic int active_window;
    alignas(64) TenantMailbox tenants[VX_MAX_WINDOWS];
} IPC_Mailbox;

typedef struct {
    IPC_Mailbox mailbox;
    int render_index;
    int write_index;
} EngineState;

typedef struct {
    alignas(64) RingPacket packets[VX_RING_SIZE];
    alignas(64) _Atomic int  ready_idx[VX_MAX_WINDOWS];
    alignas(64) _Atomic int  local_read[VX_MAX_WINDOWS];
    alignas(64) int          active_ring_slots[VX_MAX_WINDOWS][10];
    alignas(64) _Atomic uint32_t locked_mask;
} RenderRing;

extern EngineState g_engine;
extern RenderRing  g_ring;


typedef struct {
    alignas(64) _Atomic uint32_t state;
    _Atomic uint32_t flags;
    _Atomic int64_t  census_live;
    _Atomic int64_t  census_total;
    char             name[VX_EXEC_NAME_MAX];
} ExecSlot;

typedef struct {
    alignas(64) ExecSlot slots[VX_EXEC_MAX];
} ExecRegistry;

typedef struct {
    alignas(64) _Atomic uint64_t event_seq;
    _Atomic int      plate_init;
    _Atomic int      atexit_installed;
    uint8_t*         plate_map;
    uint32_t         event_ring[VX_TRACE_RING_CAP];
} TraceState;


typedef struct {
    alignas(64) _Atomic uint32_t state;
    void*            lib;
    vmath_thread_t   thread;
    _Atomic int32_t  thread_done;
    _Atomic int32_t  joined;
    _Atomic int32_t  abort;
    _Atomic uint64_t in_a;
    _Atomic uint64_t in_b;
    _Atomic uint64_t in_c;
    _Atomic uint64_t yield_seq;
    int32_t          held_idx;
} CoroSlot;

typedef struct {
    alignas(64) CoroSlot     slots[VX_CORO_MAX];
    _Atomic int32_t          shutdown;
} CoroRegistry;

extern EngineState  g_engine;
extern RenderRing   g_ring;
extern ExecRegistry g_execs;
extern TraceState   g_trace;
extern CoroRegistry g_coros;
