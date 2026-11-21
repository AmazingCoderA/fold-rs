import pefile, struct, os, sys

EXE = r"C:\Users\neon\Downloads\03_Fold\03_Fold.exe"
OUT = r"D:\2026RustLev\RustFold\analysis\res"
os.makedirs(OUT, exist_ok=True)
pe = pefile.PE(EXE)

def rva_to_off(rva):
    return pe.get_offset_from_rva(rva)

def res_name(e):
    if e.name is not None:
        return e.name.decode("utf-8", "replace")
    return str(e.id)

def walk(entry, path):
    for e in entry.entries:
        nm = res_name(e)
        if hasattr(e, "directory"):
            walk(e.directory, path + [nm])
        elif hasattr(e, "data"):
            rva = e.data.struct.OffsetToData
            size = e.data.struct.Size
            off = rva_to_off(rva)
            blob = pe.__data__[off:off+size]
            full = "/".join(path + [nm])
            fn = os.path.join(OUT, full.replace("/", "__") + ".bin")
            open(fn, "wb").write(blob)
            print("%-70s rva=%08X size=%-6d file=%s" % (full, rva, size, os.path.basename(fn)))

print("=== ALL RESOURCES ===")
walk(pe.DIRECTORY_ENTRY_RESOURCE, [])

print("\n=== RT_STRING (type 6) DECODED ===")
# pefile groups RT_STRING entries under id 6
rt_string = None
for e in pe.DIRECTORY_ENTRY_RESOURCE.entries:
    if e.id == 6 and hasattr(e, "directory"):
        rt_string = e.directory
        break

if rt_string:
    for block in rt_string.entries:
        if not hasattr(block, "directory"):
            continue
        block_id = block.id
        for lang in block.directory.entries:
            if not hasattr(lang, "data"):
                continue
            rva = lang.data.struct.OffsetToData
            size = lang.data.struct.Size
            off = rva_to_off(rva)
            blob = pe.__data__[off:off+size]
            pos = 0
            strings = []
            for i in range(16):
                if pos + 2 > len(blob):
                    break
                ln = struct.unpack_from("<H", blob, pos)[0]
                pos += 2
                if pos + ln*2 > len(blob):
                    break
                s = blob[pos:pos+ln*2].decode("utf-16le", "replace")
                pos += ln*2
                if s:
                    strings.append((block_id*16 + i - 16, s))
            print("\n-- block %d --" % block_id)
            for ident, s in strings:
                print("  %5d: %s" % (ident, repr(s)))
