
BIN = "target/release/alm"

OUT = "target/run_out.txt"
ERR = "target/run_err.txt"
BERR = "target/build_err.txt"

CASES_DIR = "cases"
LOCK_DIR = "lock"
DIFF_DIR = "diff"

os.execute("mkdir -p target")

function read_file(path)
    local f = io.open(path, "r")
    if not f then return "" end
    local content = f:read("*a")
    f:close()
    return content
end

function copy_file(src, dst)
    local i = io.open(src, "rb")
    if not i then return false end
    local data = i:read("*a")
    i:close()
    local o = io.open(dst, "wb")
    if not o then return false end
    o:write(data)
    o:close()
    return true
end

-- Pin grammar and classification rules: README.md. Unknown EXPECT_*
function parse_pins(content)
    local pins = { expect = {}, build_fail = {}, panic = {}, unknown = {} }
    for prefix, rest in content:gmatch("%-%-%s*(EXPECT[_A-Z]*):%s*([^\r\n]+)") do
        if prefix == "EXPECT" then
            table.insert(pins.expect, rest)
        elseif prefix == "EXPECT_BUILD_FAIL" then
            table.insert(pins.build_fail, rest)
        elseif prefix == "EXPECT_PANIC" then
            table.insert(pins.panic, rest)
        else
            table.insert(pins.unknown, prefix)
        end
    end
    pins.args = content:match("%-%-%s*ARGS:%s*([^\r\n]+)")
    pins.flags = content:match("%-%-%s*FLAGS:%s*([^\r\n]+)")
    return pins
end

function scan_corpus()
    local corpus = { positive = {}, negative = {}, panic = {},
                     bad = {}, odd = {}, pins = {} }
    local p = io.popen("ls " .. CASES_DIR .. " 2>/dev/null")
    for name in p:lines() do
        if name:sub(-4) == ".lua" then
            local pins = parse_pins(read_file(CASES_DIR .. "/" .. name))
            corpus.pins[name] = pins
            local is_neg, is_pan = #pins.build_fail > 0, #pins.panic > 0
            local why
            if is_neg and is_pan then
                why = "carries both EXPECT_BUILD_FAIL and EXPECT_PANIC pins"
            elseif is_neg and #pins.expect > 0 then
                why = "negative (EXPECT_BUILD_FAIL) but also carries EXPECT pins"
            end
            if why then
                table.insert(corpus.bad, { name, why })
            elseif is_neg then
                table.insert(corpus.negative, name)
            elseif is_pan then
                table.insert(corpus.panic, name)
            else
                table.insert(corpus.positive, name)
            end
            if #pins.unknown > 0 then
                table.insert(corpus.odd, { name, table.concat(pins.unknown, ", ") })
            end
        end
    end
    p:close()
    return corpus
end

function run_case(src, args, with_backtrace, flags)
    local bt = with_backtrace and "RUST_BACKTRACE=1 " or ""
    local argstr = args and (" " .. args) or ""
    local flagstr = flags and (flags .. " ") or ""
    local res = os.execute(string.format(
        "%stimeout 120 %s %s'%s'%s > %s 2> %s", bt, BIN, flagstr, src, argstr, OUT, ERR))
    return res == true or res == 0
end

function compile_succeeded()
    return read_file(OUT):find("Success! Shared library written", 1, true) ~= nil
end

function compile_succeeded_exe()
    return read_file(OUT):find("Success! executable written", 1, true) ~= nil
end

function run_case_exe(src, flags)
    local flagstr = flags and (flags .. " ") or ""
    local res = os.execute(string.format(
        "timeout 120 %s %s--exe '%s' > %s 2> %s", BIN, flagstr, src, OUT, ERR))
    return res == true or res == 0
end

function run_exe_with(argstr)
    local res = os.execute(string.format(
        "timeout 120 ./alm_out%s > %s 2> %s", argstr, OUT, ERR))
    return res == true or res == 0
end

-- report goes to stderr and never touches EXPECT pins).
function script_stdout()
    local lines = {}
    for line in read_file(OUT):gmatch("[^\r\n]+") do
        if not line:find("Success! Shared library written", 1, true)
            and not line:find("Success! executable written", 1, true) then
            table.insert(lines, line)
        end
    end
    return table.concat(lines, "\n")
end

function first_error(path)
    local s = read_file(path)
    local msg = s:match("panicked at.-\n%s*([^\n]+)")
    if not msg then
        for line in s:gmatch("[^\r\n]+") do
            if line:find("Error", 1, true) then msg = line break end
        end
    end
    return msg
        or s:match("([^\r\n]+)")
        or "(no error output)"
end

-- The lock oracle normalizes the DIFile directory out of the compared
-- IR: the compiler truthfully embeds the source's absolute directory
-- (backend.rs DebugMeta), which would make every lock machine-absolute
-- and every differently-named checkout read as 100% drift. Both sides
-- of the comparison are normalized (the freshly captured IR here, the
-- lock on disk at diff time in run.lua), so absolute-path locks from
-- before the normalization keep passing, and reset.lua writes portable
-- ones. The filename stays: it names the case and does not vary.
function normalize_ir(code)
    return (code:gsub('(!DIFile%([^\n]-directory: ")([^"]*)(")', "%1<CASES_DIR>%3"))
end

function captured_ir()
    local code = read_file("libalm_out.ll")
    if code ~= "" then return normalize_ir(code) end
    return nil
end

-- The ffi surface is --exe-only (the acyclic-destiny gate), so its
-- positives compile the standalone lane: their IR lands in
-- alm_out.ll, and the battery drives them through the exe path.
function flags_exe(flags)
    return flags ~= nil and flags:find("--exe", 1, true) ~= nil
end

function captured_ir_exe()
    local code = read_file("alm_out.ll")
    if code ~= "" then return normalize_ir(code) end
    return nil
end
