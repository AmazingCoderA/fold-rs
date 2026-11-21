import re, struct

d = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()

# Scan for: shortstring name + word + dword code address in the method-table region
region = d[0x67D00:0x67F20]
base = 0x67D00
found = []
for i in range(len(region)-8):
    n = region[i]
    if not (2 <= n <= 40):
        continue
    name = region[i+1:i+1+n]
    if not all(32 <= c < 127 for c in name):
        continue
    w = struct.unpack_from("<H", region, i+1+n)[0]
    addr = struct.unpack_from("<I", region, i+3+n)[0]
    if 0x401000 <= addr < 0x4A0000 and re.fullmatch(rb"[A-Za-z_][A-Za-z0-9_]*", name):
        found.append((base+i, name.decode(), w, addr))
for off, name, w, addr in found:
    print("%08X  %-28s word=%04X  addr=%08X (RVA %06X)" % (off, name, w, addr, addr-0x400000))
