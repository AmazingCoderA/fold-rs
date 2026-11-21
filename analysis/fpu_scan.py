import struct
from capstone import *
from capstone.x86 import *

CODE = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()
BASE = 0x400000
md = Cs(CS_ARCH_X86, CS_MODE_32)
md.detail = True

md.skipdata = True
insns = list(md.disasm(CODE[0x60000:0x6A000], BASE + 0x60000))
print("instructions:", len(insns))

def rd(addr, size):
    return CODE[addr-BASE:addr-BASE+size]

def fmtimm(x):
    try:
        f = struct.unpack("<f", struct.pack("<I", x & 0xFFFFFFFF))[0]
        return "f32=%.6g" % f
    except Exception:
        return ""

out = open(r"D:\2026RustLev\RustFold\analysis\fpu_report.txt", "w", encoding="utf-8")
fpu = [i for i, ins in enumerate(insns) if ins.mnemonic.startswith("f") and ins.mnemonic not in ("fwait",)]
print("FPU instructions:", len(fpu), file=out)
for idx in fpu:
    ins = insns[idx]
    lo = max(0, idx-6); hi = min(len(insns), idx+4)
    out.write("\n--- %08X ---\n" % ins.address)
    for j in range(lo, hi):
        k = insns[j]
        mark = ">>" if j == idx else "  "
        line = "%s %08X %-8s %s" % (mark, k.address, k.mnemonic, k.op_str)
        try:
            ops = k.operands
        except Exception:
            continue
        for op in ops:
            if op.type == X86_OP_IMM:
                nm = fmtimm(op.imm)
                if nm: line += "   ; " + nm
            if op.type == X86_OP_MEM and op.mem.base == 0 and op.mem.disp:
                a = op.mem.disp & 0xFFFFFFFF
                if BASE <= a < BASE + 0xA8000:
                    raw = rd(a, 8)
                    f64 = struct.unpack("<d", raw[:8])[0]
                    if 1e-12 < abs(f64) < 1e12:
                        line += "   ; double=%.10g" % f64
        out.write(line + "\n")
out.close()
print("report written")
