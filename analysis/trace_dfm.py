import struct, sys

d = open(r"D:\2026RustLev\RustFold\analysis\res_unpacked\10__TFORM1__0.bin", "rb").read()
log = open(r"D:\2026RustLev\RustFold\analysis\dfm_trace2.txt", "w", encoding="utf-8")
pos = 4

def ss():
    global pos
    n = d[pos]; pos += 1
    s = d[pos:pos+n].decode("cp1251", "replace"); pos += n
    return s

def val(t):
    global pos
    if t == 2: pos += 1
    elif t == 3: pos += 2
    elif t == 4: pos += 4
    elif t == 5: pos += 10
    elif t == 6:
        n = d[pos]; pos += 1 + n
    elif t == 7: ss()
    elif t in (8, 9, 13): pass
    elif t == 10:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4 + n
    elif t == 11:
        while d[pos] != 0: ss()
        pos += 1
    elif t == 12:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4 + n
    elif t == 14:
        while d[pos] != 0:
            pos += 1
            while d[pos] != 0:
                ss(); vt = d[pos]; pos += 1; val(vt)
            pos += 1
    elif t in (15,): pos += 4
    elif t in (16, 17, 19): pos += 8
    elif t == 18:
        n = struct.unpack_from("<I", d, pos)[0]; pos += 4 + n*2
    else:
        raise Exception("bad type %d at %X" % (t, pos-1))

def obj(depth):
    global pos
    flag = d[pos]; pos += 1
    cls = ss(); name = ss()
    log.write("%sOBJ %s %s flag=%d @%X\n" % ("  "*depth, cls, name, flag, pos))
    while d[pos] != 0:
        st = pos
        pn = ss()
        t = d[pos]; pos += 1
        log.write("%s  %-30s t=%d @%X\n" % ("  "*depth, pn, t, st))
        val(t)
    pos += 1
    while d[pos] != 0:
        obj(depth+1)
    pos += 1

try:
    obj(0)
except Exception as e:
    log.write("ERROR: %s at pos=%X\n" % (e, pos))
log.close()
print("done, lines:", sum(1 for _ in open(r"D:\2026RustLev\RustFold\analysis\dfm_trace2.txt", encoding="utf-8")))
