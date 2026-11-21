import sys, pefile
from capstone import *
from capstone.x86 import *

DUMP = r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin"
CODE = open(DUMP, "rb").read()
BASE = 0x400000

SYMS = {
    0x468140: "FormCreate",
    0x46805C: "begin_prf",
    0x468780: "N1Click",
    0x468934: "coora",
    0x468314: "C1Click",
    0x46843C: "paint_prf",
    0x4689EC: "end_prf",
    0x468DDC: "Delete_ProfileClick",
    0x4691D4: "UnDelete_ProfileClick",
    0x469514: "EntryHandleClick",
}

def name_for(addr):
    return SYMS.get(addr)

def dis(start_rva, count=200, out=None):
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    md.detail = True
    code = CODE[start_rva:start_rva+count*16]
    lines = []
    for ins in md.disasm(code, BASE + start_rva):
        txt = "%08X  %-20s %-8s %s" % (ins.address, ins.bytes.hex(), ins.mnemonic, ins.op_str)
        for op in ins.operands:
            if op.type == X86_OP_IMM:
                nm = name_for(op.imm)
                if nm:
                    txt += "    ; " + nm
        lines.append(txt)
        if len(lines) >= count:
            break
    s = "\n".join(lines)
    if out:
        open(out, "a", encoding="utf-8").write("=== RVA %06X ===\n" % start_rva + s + "\n\n")
    return s

if __name__ == "__main__":
    for arg in sys.argv[1:]:
        parts = arg.split(":")
        rva = int(parts[0], 16)
        n = int(parts[1]) if len(parts) > 1 else 120
        print("=== RVA %06X ===" % rva)
        print(dis(rva, n))
