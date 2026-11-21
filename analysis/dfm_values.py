"""Правильный парсер бинарного DFM: сохраняет ЧИСЛОВЫЕ значения свойств.

Старый parse_dfm.py возвращал None для типов 2/3/4, поэтому координаты
и размеры контролов были потеряны. Здесь они выводятся полностью.
"""
import struct

PATH = r"D:\2026RustLev\RustFold\analysis\res_unpacked\10__TFORM1__0.bin"
d = open(PATH, "rb").read()
assert d[:4] == b"TPF0", d[:4]
pos = 4


def shortstr():
    global pos
    n = d[pos]
    pos += 1
    s = d[pos : pos + n].decode("cp1251", "replace")
    pos += n
    return s


def signed(t, raw):
    return int.from_bytes(raw, "little", signed=True)


def read_value(t):
    global pos
    if t == 0:
        return None
    if t == 1:  # list of values
        out = []
        while d[pos] != 0:
            t2 = d[pos]
            pos += 1
            out.append((t2, read_value(t2)))
        pos += 1
        return out
    if t == 2:
        v = d[pos]
        pos += 1
        return ("i8", signed(t, bytes([v])), v)
    if t == 3:
        raw = d[pos : pos + 2]
        pos += 2
        return ("i16", signed(t, raw), int.from_bytes(raw, "little"))
    if t == 4:
        raw = d[pos : pos + 4]
        pos += 4
        return ("i32", signed(t, raw), int.from_bytes(raw, "little"))
    if t == 5:
        pos += 10
        return "extended"
    if t == 6:
        return shortstr()
    if t == 7:
        return shortstr()
    if t == 8:
        return False
    if t == 9:
        return True
    if t == 10:
        n = struct.unpack_from("<I", d, pos)[0]
        pos += 4 + n
        return "<binary %d>" % n
    if t == 11:  # set of identifiers
        out = []
        while d[pos] != 0:
            out.append(shortstr())
        pos += 1
        return out
    if t == 12:
        n = struct.unpack_from("<I", d, pos)[0]
        pos += 4
        s = d[pos : pos + n].decode("cp1251", "replace")
        pos += n
        return s
    if t == 13:
        return "nil"
    if t == 14:  # collection
        out = []
        while d[pos] != 0:
            pos += 1  # item marker 0x01
            item = []
            while True:
                tc = d[pos]
                pos += 1
                if tc == 0:
                    break
                pname = shortstr()
                item.append((pname, tc, read_value(tc)))
            out.append(item)
        pos += 1
        return out
    if t == 15:
        raw = d[pos : pos + 4]
        pos += 4
        return ("single", struct.unpack_from("<f", raw)[0])
    if t == 16:
        pos += 8
        return "currency"
    if t == 17:
        pos += 8
        return "date"
    if t == 18:
        n = struct.unpack_from("<I", d, pos)[0]
        pos += 4
        s = d[pos : pos + n * 2].decode("utf-16le", "replace")
        pos += n * 2
        return s
    if t == 19:
        raw = d[pos : pos + 8]
        pos += 8
        return ("i64", signed(t, raw), int.from_bytes(raw, "little"))
    if t == 20:
        n = struct.unpack_from("<I", d, pos)[0]
        pos += 4
        s = d[pos : pos + n].decode("utf-8", "replace")
        pos += n
        return s
    raise Exception("unknown type %d at %X" % (t, pos - 1))


def parse_object(depth):
    global pos
    flag = 0
    if d[pos] == 1:  # inherited flag
        flag = 1
        pos += 1
    cls = shortstr()
    name = shortstr()
    props = []
    while d[pos] != 0:
        pname = shortstr()
        t = d[pos]
        pos += 1
        props.append((pname, t, read_value(t)))
    pos += 1
    children = []
    while d[pos] != 0:
        children.append(parse_object(depth + 1))
    pos += 1
    return cls, name, props, children


def fmt(v):
    if isinstance(v, tuple) and len(v) == 3 and v[0] in ("i8", "i16", "i32", "i64"):
        return "%d (0x%X)" % (v[1], v[2])
    return repr(v)


obj = parse_object(0)
out = []


def emit(o, depth):
    cls, name, props, children = o
    out.append("  " * depth + "%s %s" % (cls, name))
    for pn, t, v in props:
        out.append("  " * (depth + 1) + "%-24s = %-30s [t=%d]" % (pn, fmt(v), t))
    for c in children:
        emit(c, depth + 1)


emit(obj, 0)
open(r"D:\2026RustLev\RustFold\analysis\dfm_values_out.txt", "w", encoding="utf-8").write(
    "\n".join(out)
)
print("\n".join(out))
print("total lines:", len(out), "parsed bytes:", pos, "of", len(d))
