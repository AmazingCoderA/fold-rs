import sys, pefile, hashlib, os

EXE = r"C:\Users\neon\Downloads\03_Fold\03_Fold.exe"

pe = pefile.PE(EXE, fast_load=False)
data = open(EXE, "rb").read()
print("FILE")
print("  path   :", EXE)
print("  size   :", len(data))
print("  md5    :", hashlib.md5(data).hexdigest())
print("  sha256 :", hashlib.sha256(data).hexdigest())

print("\nDOS HEADER")
print("  e_magic   :", pe.DOS_HEADER.e_magic, hex(pe.DOS_HEADER.e_magic))
print("  e_lfanew  :", hex(pe.DOS_HEADER.e_lfanew))

print("\nFILE HEADER")
fh = pe.FILE_HEADER
print("  Machine          :", hex(fh.Machine))
print("  NumberOfSections :", fh.NumberOfSections)
print("  TimeDateStamp    :", fh.TimeDateStamp)
print("  PointerToSymTab  :", hex(fh.PointerToSymbolTable))
print("  NumSymbols       :", fh.NumberOfSymbols)
print("  SizeOfOptHeader  :", fh.SizeOfOptionalHeader)
print("  Characteristics  :", hex(fh.Characteristics))

print("\nOPTIONAL HEADER")
oh = pe.OPTIONAL_HEADER
print("  Magic                  :", hex(oh.Magic))
print("  MajorLinkerVersion     :", oh.MajorLinkerVersion)
print("  MinorLinkerVersion     :", oh.MinorLinkerVersion)
print("  SizeOfCode             :", hex(oh.SizeOfCode))
print("  SizeOfInitializedData  :", hex(oh.SizeOfInitializedData))
print("  SizeOfUninitializedData:", hex(oh.SizeOfUninitializedData))
print("  AddressOfEntryPoint    :", hex(oh.AddressOfEntryPoint))
print("  BaseOfCode             :", hex(oh.BaseOfCode))
if hasattr(oh, "BaseOfData"):
    print("  BaseOfData             :", hex(oh.BaseOfData))
print("  ImageBase              :", hex(oh.ImageBase))
print("  SectionAlignment       :", hex(oh.SectionAlignment))
print("  FileAlignment          :", hex(oh.FileAlignment))
print("  SizeOfImage            :", hex(oh.SizeOfImage))
print("  SizeOfHeaders          :", hex(oh.SizeOfHeaders))
print("  Subsystem              :", oh.Subsystem, "(2=GUI,3=CUI)")
print("  DllCharacteristics     :", hex(oh.DllCharacteristics))
for n in ["SizeOfStackReserve", "SizeOfStackCommit", "SizeOfHeapReserve", "SizeOfHeapCommit"]:
    print("  %-23s:" % n, hex(getattr(oh, n)))
print("  NumberOfRvaAndSizes    :", oh.NumberOfRvaAndSizes)

print("\nDATA DIRECTORIES")
for i, d in enumerate(oh.DATA_DIRECTORY):
    if d.VirtualAddress or d.Size:
        print("  [%2d] %-24s VA=%08X Size=%08X" % (i, d.name, d.VirtualAddress, d.Size))

print("\nSECTIONS")
for s in pe.sections:
    print("  %-8s VS=%08X VA=%08X RS=%08X RP=%08X CH=%08X" % (
        s.Name.decode("latin1").rstrip("\x00"), s.Misc_VirtualSize,
        s.VirtualAddress, s.SizeOfRawData, s.PointerToRawData, s.Characteristics))

print("\nIMPORTS")
if hasattr(pe, "DIRECTORY_ENTRY_IMPORT"):
    for imp in pe.DIRECTORY_ENTRY_IMPORT:
        dll = imp.dll.decode("latin1")
        print("  DLL:", dll)
        for entry in imp.imports:
            name = entry.name.decode("latin1") if entry.name else "ordinal:%d" % entry.ordinal
            print("    %-40s hint=%s" % (name, hex(entry.hint) if entry.hint is not None else "-"))
else:
    print("  (none)")

print("\nEXPORTS")
if hasattr(pe, "DIRECTORY_ENTRY_EXPORT"):
    for exp in pe.DIRECTORY_ENTRY_EXPORT.symbols:
        print("  ordinal=%-5s name=%s rva=%s" % (exp.ordinal, exp.name, hex(exp.address) if exp.address else "-"))
else:
    print("  (none)")

print("\nRESOURCES")
def walk_resources(entry, path=""):
    if hasattr(entry, "directory"):
        name = entry.name if entry.name is not None else "id:%d" % entry.id
        walk_resources(entry.directory, path + "/" + str(name))
    else:
        for e in entry.entries:
            name = e.name if e.name is not None else "id:%d" % e.id
            if hasattr(e, "data"):
                print("  %-60s RVA=%08X size=%d" % (path + "/" + str(name), e.data.struct.OffsetToData, e.data.struct.Size))
            else:
                walk_resources(e, path + "/" + str(name))
if hasattr(pe, "DIRECTORY_ENTRY_RESOURCE"):
    walk_resources(pe.DIRECTORY_ENTRY_RESOURCE)
else:
    print("  (none)")

print("\nSTRINGS (ASCII >= 5 chars in raw file)")
import re
for m in re.finditer(rb"[ -~]{5,}", data):
    s = m.group().decode("latin1")
    print("  %08X  %s" % (m.start(), s))

print("\nSTRINGS (UTF-16LE >= 4 chars)")
for m in re.finditer(rb"(?:[ -~]\x00){4,}", data):
    s = m.group().decode("utf-16le")
    print("  %08X  %s" % (m.start(), s))
