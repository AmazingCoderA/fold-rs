import re
lines = open(r"D:\2026RustLev\RustFold\analysis\dfm_tree.txt", encoding="utf-8").read().splitlines()
for ln in lines:
    s = ln.strip()
    if re.match(r"^(T\w+\s+\w+\s\[flag)", s):
        print("\n" + s)
    elif re.match(r"^(On\w+|Caption|Text|Items|Strings)\s*=", s):
        print("   " + s)
