import struct, sys

PATH = r"D:\2026RustLev\RustFold\analysis\res_unpacked\10__TFORM1__0.bin"
d = open(PATH, "rb").read()
assert d[:4] == b"TPF0"
pos = 4

def shortstr():
    global pos
    n = d[pos]; pos += 1
    s = d[pos:pos+n].decode("cp1251", "replace"); pos += n
    return s

def skip_value(t):
    global pos
    if t == 0:            # null
        return None
    if t == 1:            # list
        out = []
        while d[pos] != 0:
            t2 = d[pos]; pos += 1
            out.append(skip_value(t2))
        pos += 1
        return out
    if t == 2: pos += 1; return None
    if t == 3: pos += 2; return None
    if t == 4: pos += 4; return None
    if t == 5: pos += 10; return None
    if t == 6:
        n = d[pos]; pos += 1
        s = d[pos:pos+n].decode("cp1251", "replace"); pos += n
        return s
    if t == 7: return shortstr()
    if t in (8, 9): return None
    if t == 10:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4
        pos += n
        return "<binary %d>" % n
    if t == 11:
        out = []
        while d[pos] != 0:
            out.append(shortstr())
        pos += 1
        return out
    if t == 12:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4
        s = d[pos:pos+n].decode("cp1251", "replace"); pos += n
        return s
    if t == 13: return None
    if t == 14:
        while d[pos] != 0:
            pos += 1  # 0x01 item marker
            while True:
                tc = d[pos]; pos += 1
                if tc == 0: break
                pname = shortstr()
                skip_value(tc)
        pos += 1
        return "<collection>"
    if t == 15: pos += 4; return None
    if t == 16: pos += 8; return None
    if t == 17: pos += 8; return None
    if t == 18:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4
        s = d[pos:pos+n*2].decode("utf-16le", "replace"); pos += n*2
        return s
    if t == 19: pos += 8; return None
    if t == 20:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4
        s = d[pos:pos+n].decode("utf-8", "replace"); pos += n
        return s
    raise Exception("unknown value type %d at %X" % (t, pos-1))

def read_value():
    global pos
    t = d[pos]; pos += 1
    return t, skip_value(t)

def parse_object(depth):
    global pos
    flag = 0
    cls = shortstr()
    name = shortstr()
    props = []
    while d[pos] != 0:
        pname = shortstr()
        open(r"D:\2026RustLev\RustFold\analysis\dfm_trace.txt", "a", encoding="utf-8").write("  " * depth + "%s.%s @%X\n" % (name, pname, pos))
        t, v = read_value()
        props.append((pname, t, v))
    pos += 1  # end of properties
    children = []
    while d[pos] != 0:
        children.append(parse_object(depth+1))
    pos += 1  # end of children
    return dict(flag=flag, cls=cls, name=name, props=props, children=children)

obj = parse_object(0)

def fmtval(t, v):
    if v is None:
        return {2:"i8",3:"i16",4:"i32",5:"ext",8:"false",9:"true",13:"nil",
                15:"single",16:"currency",17:"date",19:"i64"}.get(t, "null")
    return repr(v)

out = []
def emit(o, depth):
    out.append("  "*depth + "%s %s [flag=%d]" % (o["cls"], o["name"], o["flag"]))
    for pn, t, v in o["props"]:
        out.append("  "*(depth+1) + "%s = %s (type %d)" % (pn, fmtval(t,v), t))
    for c in o["children"]:
        emit(c, depth+1)
emit(obj, 0)
open(r"D:\2026RustLev\RustFold\analysis\dfm_tree.txt", "w", encoding="utf-8").write("\n".join(out))
print("\n".join(out[:200]))
print("...")
print("total lines:", len(out))
