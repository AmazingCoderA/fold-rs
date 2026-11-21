import struct

d = open(r"D:\2026RustLev\RustFold\analysis\memory\unpacked_image.bin", "rb").read()
BASE = 0x400000

CONSTS = [
    0x4663e4, 0x4663e8, 0x4663ec, 0x4663f0, 0x4663f4, 0x4663f8,
    0x466400, 0x466404, 0x466408, 0x46640c, 0x466410, 0x466414,
    0x466418, 0x46641c,
    0x466940, 0x466944, 0x466948, 0x46694c, 0x466950, 0x466954,
    0x466958, 0x46695c, 0x466960, 0x466964, 0x466968, 0x46696c,
    0x466970, 0x466974, 0x466978, 0x46697c,
    0x467410, 0x467414, 0x467418, 0x46741c,
    0x467424, 0x467428, 0x46742c, 0x467430, 0x467434, 0x467438,
    0x467440, 0x467444, 0x467448, 0x46744c,
    0x466150, 0x466154, 0x466158,
    0x46ae48, 0x46ae4c, 0x46ae50, 0x46ae54,
]

def ext80(off):
    b = d[off:off+10]
    if len(b) < 10:
        return None
    sign = -1 if b[9] & 0x80 else 1
    exp = ((b[9] & 0x7f) << 8) | b[8]
    mant = int.from_bytes(b[:8], "little")
    if exp == 0 and mant == 0:
        return 0.0
    if exp == 0x7fff:
        return float("inf") * sign
    try:
        return sign * float(mant) * (2.0 ** (exp - 16383 - 63))
    except OverflowError:
        return None

def safe(fmt, off):
    try:
        v = struct.unpack_from(fmt, d, off)[0]
        if abs(v) > 1e30 and abs(v) != float("inf"):
            return None
        return v
    except Exception:
        return None

def f32(off):
    return safe("<f", off)

def f64(off):
    return safe("<d", off)

for va in CONSTS:
    off = va - BASE
    raw = d[off:off+16].hex()
    def fmt(v):
        return "n/a" if v is None else "%.12g" % v
    print("%08X : %-32s f32=%-16s f64=%-20s ext80=%-20s" % (
        va, raw, fmt(f32(off)), fmt(f64(off)), fmt(ext80(off))))
