"""a sampling profiler for a running Windows process: suspends each of its
threads about a thousand times a second, reads the instruction pointer and
counts where it was, by module and by symbol.

    python tools/sample_profile.py <pid> <seconds> [out.txt]

symbols come from `nm` for modules that have a symbol table (the recompiled
DLL) and `addr2line -f` for those with DWARF (Zakuro's boot.exe), both from
the MinGW binutils on PATH (source tools/env.sh first, or set BINUTILS)."""
import bisect
import collections
import ctypes
import ctypes.wintypes as wt
import os
import subprocess
import sys
import time

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
psapi = ctypes.WinDLL("psapi", use_last_error=True)

TH32CS_SNAPTHREAD = 0x4
THREAD_ACCESS = 0x0002 | 0x0008 | 0x0040  # suspend/resume, get context, query
PROCESS_ACCESS = 0x0400 | 0x0010  # query information, vm read
CONTEXT_CONTROL = 0x00100001


class THREADENTRY32(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("cntUsage", wt.DWORD), ("th32ThreadID", wt.DWORD),
                ("th32OwnerProcessID", wt.DWORD), ("tpBasePri", wt.LONG), ("tpDeltaPri", wt.LONG),
                ("dwFlags", wt.DWORD)]


class MODULEINFO(ctypes.Structure):
    _fields_ = [("lpBaseOfDll", ctypes.c_void_p), ("SizeOfImage", wt.DWORD), ("EntryPoint", ctypes.c_void_p)]


k32.OpenThread.restype = wt.HANDLE
k32.OpenProcess.restype = wt.HANDLE
k32.CreateToolhelp32Snapshot.restype = wt.HANDLE


def threads(pid):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
    entry = THREADENTRY32()
    entry.dwSize = ctypes.sizeof(entry)
    found = []
    ok = k32.Thread32First(snap, ctypes.byref(entry))
    while ok:
        if entry.th32OwnerProcessID == pid:
            found.append(entry.th32ThreadID)
        ok = k32.Thread32Next(snap, ctypes.byref(entry))
    k32.CloseHandle(snap)
    return found


def modules(pid):
    process = k32.OpenProcess(PROCESS_ACCESS, False, pid)
    handles = (ctypes.c_void_p * 1024)()
    needed = wt.DWORD()
    psapi.EnumProcessModulesEx(process, handles, ctypes.sizeof(handles), ctypes.byref(needed), 3)
    found = []
    for handle in handles[: needed.value // ctypes.sizeof(ctypes.c_void_p)]:
        name = ctypes.create_unicode_buffer(1024)
        psapi.GetModuleFileNameExW(process, ctypes.c_void_p(handle), name, 1024)
        info = MODULEINFO()
        psapi.GetModuleInformation(process, ctypes.c_void_p(handle), ctypes.byref(info), ctypes.sizeof(info))
        found.append((info.lpBaseOfDll, info.lpBaseOfDll + info.SizeOfImage, name.value))
    k32.CloseHandle(process)
    return sorted(found)


def rip(handle):
    """the thread's instruction and stack pointers."""
    # CONTEXT is 1232 bytes and has to be 16-byte aligned; Rsp is at 0x98,
    # Rip at 0xF8
    raw = ctypes.create_string_buffer(1232 + 16)
    base = (ctypes.addressof(raw) + 15) & ~15
    ctypes.c_uint32.from_address(base + 0x30).value = CONTEXT_CONTROL
    if not k32.GetThreadContext(handle, ctypes.c_void_p(base)):
        return None
    return ctypes.c_uint64.from_address(base + 0xF8).value, ctypes.c_uint64.from_address(base + 0x98).value


# with FOCUS=<text>, samples whose function's name holds it also get their
# callers guessed: values on the top of the stack that point into an .exe's
# code are taken for return addresses
FOCUS = os.environ.get("FOCUS")
STACK_SCAN = 1024


def stack_words(process, rsp):
    buffer = ctypes.create_string_buffer(STACK_SCAN)
    read = ctypes.c_size_t()
    if not k32.ReadProcessMemory(process, ctypes.c_void_p(rsp), buffer, STACK_SCAN, ctypes.byref(read)):
        return []
    return [int.from_bytes(buffer.raw[i:i + 8], "little") for i in range(0, read.value - 7, 8)]


def tool(name):
    return os.path.join(os.environ["BINUTILS"], name) if os.environ.get("BINUTILS") else name


def nm_symbols(path):
    """(rva, name) of the text symbols, from the image base nm reports."""
    out = subprocess.run([tool("nm"), "--defined-only", path], capture_output=True, text=True).stdout
    image_base = None
    head = subprocess.run([tool("objdump"), "-p", path], capture_output=True, text=True).stdout
    for line in head.splitlines():
        if line.startswith("ImageBase"):
            image_base = int(line.split()[1], 16)
    symbols = []
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[1] in "tT":
            symbols.append((int(parts[0], 16) - image_base, parts[2]))
    symbols.sort()
    return symbols, image_base


def main():
    pid, seconds = int(sys.argv[1]), float(sys.argv[2])
    out = open(sys.argv[3], "w") if len(sys.argv) > 3 else sys.stdout
    mods = modules(pid)
    exe = next(((lo, hi, path) for lo, hi, path in mods if path.lower().endswith(".exe")), None)
    process = k32.OpenProcess(PROCESS_ACCESS, False, pid)
    stacks = collections.defaultdict(list)
    handles = {}
    hits = collections.Counter()
    by_thread = collections.Counter()
    end = time.perf_counter() + seconds
    samples = 0
    listed = []
    while time.perf_counter() < end:
        # listing the threads costs more than sampling them
        if samples % 200 == 0:
            listed = threads(pid)
        for tid in listed:
            if tid not in handles:
                handles[tid] = k32.OpenThread(THREAD_ACCESS, False, tid)
            handle = handles[tid]
            if k32.SuspendThread(handle) == 0xFFFFFFFF:
                continue
            registers = rip(handle)
            words = stack_words(process, registers[1]) if registers and FOCUS and exe else []
            k32.ResumeThread(handle)
            if registers is not None:
                pc = registers[0]
                hits[(tid, pc)] += 1
                if words:
                    stacks[pc].append([w for w in words if exe[0] <= w < exe[1]][:16])
        samples += 1
        time.sleep(0.0005)
    starts = [m[0] for m in mods]

    def module_of(pc):
        i = bisect.bisect_right(starts, pc) - 1
        if i >= 0 and pc < mods[i][1]:
            return mods[i]
        return None

    # a thread that never leaves a wait is idle, keep the busy ones
    per_thread = collections.defaultdict(collections.Counter)
    for (tid, pc), n in hits.items():
        per_thread[tid][pc] += n
    busy = {tid for tid, pcs in per_thread.items() if len(pcs) > 3}

    by_module = collections.Counter()
    by_symbol = collections.Counter()
    tables = {}
    pending_dwarf = collections.defaultdict(list)
    inlined = collections.Counter()
    for (tid, pc), n in hits.items():
        if tid not in busy:
            continue
        module = module_of(pc)
        name = os.path.basename(module[2]) if module else "?"
        by_module[(tid, name)] += n
        if module is None:
            by_symbol[("?", f"0x{pc:X}")] += n
            continue
        path = module[2]
        if name.lower().endswith(".dll") and "Windows" not in path:
            if path not in tables:
                tables[path] = nm_symbols(path)
            symbols, _ = tables[path]
            rva = pc - module[0]
            i = bisect.bisect_right(symbols, (rva, "\xff")) - 1
            by_symbol[(name, symbols[i][1] if i >= 0 else f"+0x{rva:X}")] += n
        elif name.lower().endswith(".exe"):
            pending_dwarf[path].append((pc - module[0], n))
        else:
            by_symbol[(name, "")] += n
    for path, items in pending_dwarf.items():
        _, image_base = nm_symbols(path)
        addresses = [f"0x{image_base + rva:X}" for rva, _ in items]
        # -i gives the chain of functions inlined into each other, the last
        # the one really compiled, which is what a sample is charged to
        chains = []
        for chunk in range(0, len(addresses), 500):
            lines = subprocess.run([tool("addr2line"), "-a", "-f", "-i", "-C", "-e", path] + addresses[chunk:chunk + 500],
                                   capture_output=True, text=True).stdout.splitlines()
            for line in lines:
                if line.startswith("0x") and all(c in "0123456789abcdefABCDEFx" for c in line):
                    chains.append([])
                elif chains and not (":" in line and line.rsplit(":", 1)[-1].split()[0].isdigit() or line.startswith("??:")):
                    chains[-1].append(line)
        for (rva, n), chain in zip(items, chains):
            outer = chain[-1] if chain else "?"
            by_symbol[(os.path.basename(path), outer)] += n
            if len(chain) > 1:
                inlined[(os.path.basename(path), outer, chain[0])] += n

    total = sum(n for (tid, _), n in hits.items() if tid in busy)
    print(f"{samples} rounds, {total} samples on {len(busy)} busy threads", file=out)
    print("\nby thread and module:", file=out)
    for (tid, name), n in by_module.most_common(30):
        print(f"  {tid:>6} {name:<40} {100 * n / total:5.1f}%", file=out)
    print("\nby symbol:", file=out)
    for (name, symbol), n in by_symbol.most_common(60):
        print(f"  {100 * n / total:5.1f}%  {name:<28} {symbol[:150]}", file=out)
    print("\ninlined into (innermost first):", file=out)
    for (name, outer, inner), n in inlined.most_common(40):
        print(f"  {100 * n / total:5.1f}%  {outer[:90]}  <-  {inner[:70]}", file=out)

    if FOCUS and exe:
        # the leaf of each sample, then the callers its stack seems to hold
        _, image_base = nm_symbols(exe[2])
        wanted = sorted({w for frames in stacks.values() for frame in frames for w in frame} | set(stacks))
        names = {}
        for chunk in range(0, len(wanted), 400):
            part = wanted[chunk:chunk + 400]
            lines = subprocess.run([tool("addr2line"), "-a", "-f", "-i", "-C", "-e", exe[2]]
                                   + [f"0x{w - exe[0] + image_base:X}" for w in part], capture_output=True, text=True).stdout.splitlines()
            current, chain = None, []
            for line in lines + ["0x0"]:
                if line.startswith("0x") and all(c in "0123456789abcdefABCDEFx" for c in line):
                    if current is not None:
                        names[current] = chain[-1] if chain else "?"
                    current, chain = int(line, 16) - image_base + exe[0], []
                elif not (":" in line and line.rsplit(":", 1)[-1].split()[0].isdigit() or line.startswith("??:")):
                    chain.append(line)
        callers = collections.Counter()
        for pc, frames in stacks.items():
            leaf = names.get(pc, "?")
            if FOCUS not in leaf:
                continue
            for frame in frames:
                seen, path = set(), []
                for word in frame:
                    name = names.get(word, "?")
                    if name == "?" or name in seen or "hash" in name.lower() or name.startswith("<hashbrown") or name.startswith("core::"):
                        continue
                    seen.add(name)
                    path.append(name)
                    if len(path) == 3:
                        break
                callers[(leaf, " <- ".join(p[:70] for p in path))] += 1
        focus_total = sum(callers.values())
        print(f"\ncallers of '{FOCUS}' ({focus_total} samples, guessed from the stack):", file=out)
        for (leaf, path), n in callers.most_common(25):
            print(f"  {100 * n / max(focus_total, 1):5.1f}%  {leaf[:60]}  <-  {path}", file=out)


if __name__ == "__main__":
    main()
