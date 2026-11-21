import sys
txt = open(r"D:\2026RustLev\RustFold\analysis\callgraph_report.txt", encoding="utf-8").read()
blocks = txt.split("\n\n===== ")
for want in sys.argv[1:]:
    for b in blocks:
        if b.startswith(want + " "):
            print("===== " + b.rstrip())
            print()
            break
    else:
        print("NOT FOUND", want)
