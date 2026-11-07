-- P11 cdef round-trip: the ffi.cdef statement verb compiles today as a
-- LoadNull placeholder; the declaration string is parsed, arity- and
-- type-checked, then discarded at lowering.
-- Run (probe mode is --lib and the ffi gate refuses it; compile
-- exe-side and archive by hand):
--   target/release/alm --exe probes/p11_cdef_roundtrip.lua
--   cp alm_out.ll target/probe/p11_cdef_roundtrip.ll
-- Verdicts: compiles clean; the .ll shows one `inttoptr i64 0 to ptr`
-- for the cdef statement and nothing else consuming the string. The
-- before-picture for the declaration-registry work.

local ffi = require("ffi")
ffi.cdef("struct point { int x; int y; };")
