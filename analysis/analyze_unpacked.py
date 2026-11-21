import pefile, struct, os, re, sys

EXE = r"D:\2026RustLev\RustFold\analysis\memory\unpacked.exe"
OUT = r"D:\2026RustLev\RustFold\analysis\res_unpacked"
os.makedirs(OUT, exist_ok=True)
pe = pefile.PE(EXE)

# Dump all resources by walking the tree
def name_of(e):
    return e.name.decode("utf-8", "replace") if e.name is not None else str(e.id)

def walk(directory, path):
    for e in directory.entries:
        nm = name_of(e)
        if hasattr(e, "directory"):
            walk(e.directory, path + [nm])
        elif hasattr(e, "data"):
            off = pe.get_offset_from_rva(e.data.struct.OffsetToData)
            size = e.data.struct.Size
            blob = pe.__data__[off:off+size]
            full = "/".join(path + [nm])
            fn = os.path.join(OUT, full.replace("/", "__") + ".bin")
            open(fn, "wb").write(blob)
            print("RES %-55s rva=%08X size=%d" % (full, e.data.struct.OffsetToData, size))

walk(pe.DIRECTORY_ENTRY_RESOURCE, [])

# Decode RT_STRING blocks
print("\n=== STRING TABLE ===")
for t in pe.DIRECTORY_ENTRY_RESOURCE.entries:
    if t.id == 6 and hasattr(t, "directory"):
        for block in t.directory.entries:
            for lang in block.directory.entries:
                if not hasattr(lang, "data"):
                    continue
                off = pe.get_offset_from_rva(lang.data.struct.OffsetToData)
                blob = pe.__data__[off:off+lang.data.struct.Size]
                pos = 0
                for i in range(16):
                    if pos+2 > len(blob): break
                    ln = struct.unpack_from("<H", blob, pos)[0]; pos += 2
                    if pos+ln*2 > len(blob): break
                    s = blob[pos:pos+ln*2].decode("utf-16le", "replace"); pos += ln*2
                    if s:
                        print("%6d: %s" % ((block.id-1)*16 + i + 1, s))

# ASCII strings from the unpacked image (cp1251-friendly output via repr)
print("\n=== ASCII STRINGS (filtered, len>=4) ===")
img = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()
seen = set()
for m in re.finditer(rb"[ -~]{4,}", img):
    s = m.group().decode("latin1")
    if s in seen: continue
    seen.add(s)
    if len(s) < 5: continue
    # Focus on potentially meaningful strings
    if re.search(r"[A-Za-z]{4}", s) or any(c in s for c in "._"):
        print("%08X %s" % (m.start(), s))
