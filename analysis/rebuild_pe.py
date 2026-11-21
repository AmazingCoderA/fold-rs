import pefile, struct, os, math

SRC = r"C:\Users\neon\Downloads\03_Fold\03_Fold.exe"
DUMP = r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin"
OUT = r"D:\2026RustLev\RustFold\analysis\memory\unpacked.exe"

pe = pefile.PE(SRC)
img = open(DUMP, "rb").read()
assert len(img) == 0xA8000, len(img)

FA = pe.OPTIONAL_HEADER.FileAlignment
SA = pe.OPTIONAL_HEADER.SectionAlignment

# Work on a bytearray copy of the original headers (first SizeOfHeaders bytes)
out = bytearray(pe.OPTIONAL_HEADER.SizeOfHeaders)
orig = open(SRC, "rb").read()
out[:] = orig[:pe.OPTIONAL_HEADER.SizeOfHeaders]

# Place every section's virtual contents into a fresh raw layout
cursor = pe.OPTIONAL_HEADER.SizeOfHeaders
sections = []
for s in pe.sections:
    vs = s.Misc_VirtualSize
    va = s.VirtualAddress
    raw_size = ((vs + FA - 1) // FA) * FA
    data = img[va:va+vs]
    sections.append((s, va, vs, cursor, raw_size, data))
    cursor += raw_size

# Update section headers in the new file
for i, (s, va, vs, raw_ptr, raw_size, data) in enumerate(sections):
    off = pe.FILE_HEADER.SizeOfOptionalHeader + 0x18 + i*0x28  # relative to PE signature+4? handled below
    # PE header starts at e_lfanew; section table = e_lfanew + 4 + 20 + SizeOfOptionalHeader
    sec_table = pe.DOS_HEADER.e_lfanew + 4 + 20 + pe.FILE_HEADER.SizeOfOptionalHeader + i*40
    struct.pack_into("<I", out, sec_table + 8, vs)      # VirtualSize
    struct.pack_into("<I", out, sec_table + 16, raw_size)   # SizeOfRawData
    struct.pack_into("<I", out, sec_table + 20, raw_ptr)    # PointerToRawData

# Assemble final file
final = bytearray(out)
for s, va, vs, raw_ptr, raw_size, data in sections:
    if len(final) < raw_ptr + raw_size:
        final.extend(b"\x00" * (raw_ptr + raw_size - len(final)))
    final[raw_ptr:raw_ptr+len(data)] = data

# Fix SizeOfImage just in case
struct.pack_into("<I", final, pe.DOS_HEADER.e_lfanew + 4 + 20 + 56, 0xA8000)

open(OUT, "wb").write(final)
print("wrote", OUT, len(final), "bytes")

# Verify with pefile
p2 = pefile.PE(OUT)
print("\nSections in rebuilt file:")
for s in p2.sections:
    print("  %-8s VS=%08X VA=%08X RS=%08X RP=%08X" % (
        s.Name.decode("latin1").rstrip("\x00"), s.Misc_VirtualSize,
        s.VirtualAddress, s.SizeOfRawData, s.PointerToRawData))

print("\nImports in rebuilt file:")
if hasattr(p2, "DIRECTORY_ENTRY_IMPORT"):
    for imp in p2.DIRECTORY_ENTRY_IMPORT:
        dll = imp.dll.decode("latin1")
        names = []
        for e in imp.imports:
            names.append(e.name.decode("latin1") if e.name else "ord:%d" % e.ordinal)
        print("  %-20s %d funcs: %s" % (dll, len(names), ", ".join(names[:25])))
else:
    print("  (none)")
