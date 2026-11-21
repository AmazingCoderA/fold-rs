from capstone import *
from capstone.x86 import *
import struct, collections

CODE = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()
BASE = 0x400000
md = Cs(CS_ARCH_X86, CS_MODE_32)
md.detail = True

SEEDS = {
    0x468140: "FormCreate", 0x46805C: "begin_prf", 0x468780: "N1Click",
    0x468934: "coora", 0x468314: "C1Click", 0x46843C: "paint_prf",
    0x4689EC: "end_prf", 0x468DDC: "Delete_ProfileClick",
    0x4691D4: "UnDelete_ProfileClick", 0x469514: "EntryHandleClick",
}
APP_LO, APP_HI = 0x460000, 0x46A000

def at(va):
    rva = va - BASE
    return CODE[rva:rva+4096]

def disasm_func(va, maxins=2000):
    out = []
    for ins in md.disasm(at(va), va):
        out.append(ins)
        if ins.mnemonic == "ret" and len(out) > 3:
            break
        if len(out) >= maxins:
            break
    return out

funcs = {}
queue = collections.deque(SEEDS.keys())
while queue:
    va = queue.popleft()
    if va in funcs:
        continue
    insns = disasm_func(va)
    funcs[va] = insns
    for ins in insns:
        if ins.mnemonic in ("call", "jmp"):
            for op in ins.operands:
                if op.type == X86_OP_IMM and APP_LO <= op.imm < APP_HI:
                    if op.imm not in funcs:
                        queue.append(op.imm)

print("functions found:", len(funcs))
out = open(r"D:\2026RustLev\RustFold\analysis\callgraph_report.txt", "w", encoding="utf-8")
fpu_report = open(r"D:\2026RustLev\RustFold\analysis\fpu_clean.txt", "w", encoding="utf-8")

for va, insns in sorted(funcs.items()):
    nm = SEEDS.get(va, "")
    out.write("\n\n===== %08X %s =====\n" % (va, nm))
    for ins in insns:
        line = "%08X  %-8s %s" % (ins.address, ins.mnemonic, ins.op_str)
        for op in ins.operands:
            if op.type == X86_OP_IMM and op.imm in SEEDS:
                line += "   ; " + SEEDS[op.imm]
            if op.type == X86_OP_IMM and APP_LO <= op.imm < APP_HI:
                line += "   ; -> %08X" % op.imm
            if op.type == X86_OP_MEM and op.mem.base == 0 and op.mem.disp:
                a = op.mem.disp & 0xFFFFFFFF
                if BASE <= a < BASE + 0xA8000:
                    off = a - BASE
                    if off + 8 <= len(CODE):
                        f = struct.unpack("<d", CODE[off:off+8])[0]
                        if 1e-9 < abs(f) < 1e12:
                            line += "   ; dbl=%.10g" % f
        out.write(line + "\n")
        if ins.mnemonic.startswith("f") and ins.mnemonic not in ("fwait",):
            fpu_report.write("%08X %-22s %-8s %s ; %s\n" % (ins.address, nm, ins.mnemonic, ins.op_str, va_to_name(va) if False else ""))
out.close()
fpu_report.close()
print("reports written")
