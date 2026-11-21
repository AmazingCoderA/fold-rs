import pefile, struct, io

EXE = r"D:\2026RustLev\RustFold\analysis\memory\unpacked.exe"
pe = pefile.PE(EXE)
out = io.open(r"D:\2026RustLev\RustFold\analysis\strings_unpacked.txt", "w", encoding="utf-8")
for t in pe.DIRECTORY_ENTRY_RESOURCE.entries:
    if t.id != 6 or not hasattr(t, "directory"):
        continue
    for block in t.directory.entries:
        for lang in block.directory.entries:
            if not hasattr(lang, "data"):
                continue
            off = pe.get_offset_from_rva(lang.data.struct.OffsetToData)
            blob = pe.__data__[off:off + lang.data.struct.Size]
            pos = 0
            for i in range(16):
                if pos + 2 > len(blob):
                    break
                ln = struct.unpack_from("<H", blob, pos)[0]
                pos += 2
                if pos + ln * 2 > len(blob):
                    break
                s = blob[pos:pos + ln * 2].decode("utf-16le", "replace")
                pos += ln * 2
                if s:
                    out.write("%6d: %s\n" % ((block.id - 1) * 16 + i + 1, s))
out.close()
print("done")
