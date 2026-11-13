#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

CC=${CC:-clang}

"$CC" -shared -fPIC -O2 -Wall -Wextra -std=gnu11 -I .. \
    ../sys_sync.c \
    ../state_globals.c \
    ../mailbox.c \
    ../ring_stream.c \
    ../mailbox_accessors.c \
    ../alm_tenant.c \
    ../alm_coro.c \
    ../str_pool.c \
    -o libalm_async.so -lpthread -ldl

for prog in probe probe_1lane probe_tenant probe_backpressure probe_recycle; do
    rustc --edition 2024 -O "$prog.rs" -o "$prog" \
        -C link-arg=-L. \
        -C link-arg=-lalm_async \
        -C link-arg=-lpthread \
        -C link-arg=-Wl,-rpath,'$ORIGIN'
done

echo "built: libalm_async.so probe probe_1lane probe_tenant probe_backpressure probe_recycle"
