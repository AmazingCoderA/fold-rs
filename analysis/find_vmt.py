import re, struct

d = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()
BASE = 0x400000
hits = [m.start() for m in re.finditer(rb"\x06TForm1", d)]
print("shortstring 'TForm1' hits:", [hex(x) for x in hits])

for h in hits:
    print("\n=== around %X (RVA %X) ===" % (h, h))
    # dump 0xC0 bytes before and after
    lo = max(0, h-0xC0)
    chunk = d[lo:h+0x100]
    for i in range(0, len(chunk), 16):
        b = chunk[i:i+16]
        print("%08X  %-47s  %s" % (lo+i, " ".join("%02X" % x for x in b),
              "".join(chr(x) if 32 <= x < 127 else "." for x in b)))

# Also look for exact ASCII without length prefix
hits2 = [m.start() for m in re.finditer(rb"TForm1\x00", d)]
print("\nASCIIZ TForm1 hits:", [hex(x) for x in hits2][:20])
