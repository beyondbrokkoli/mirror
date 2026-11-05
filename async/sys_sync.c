
#include "sys_sync.h"
#include <string.h> /* the pthread_create failure sentinel */

EXPORT void vx_spin_wait(int* spin_count) {
    if (*spin_count < 1000) {
        _mm_pause();
    } else if (*spin_count < 2000) {
        #if defined(_WIN32)
            SwitchToThread();
        #else
            sched_yield();
        #endif
    } else {
        SLEEP_MS(1);
    }
    (*spin_count)++;
}


EXPORT vmath_thread_t vx_thread_start(void* (*func)(void*), void* arg) {
    pthread_t thread;
    if (pthread_create(&thread, NULL, func, arg) != 0) {
        memset(&thread, 0, sizeof(thread));
    }
    return thread;
}

EXPORT void vx_thread_join(vmath_thread_t thread) {
    if (thread == 0) return;
    pthread_join(thread, NULL);
}
