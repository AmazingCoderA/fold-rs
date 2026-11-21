from PIL import Image
import numpy as np
from collections import Counter

P = r"C:\Users\neon\Documents\ShareX\Screenshots\2026-11\QuickLook_K7rvnrxRMe.png"
im = Image.open(P).convert("RGB")
a = np.asarray(im).astype(int)
H, W, _ = a.shape
print(f"image {W}x{H}")

# ---------- color at representative points ----------
pts = [
    (0,0),(2,2),(5,5),(8,40),(100,10),(450,10),(893,10),
    (0,40),(5,40),(50,40),(100,40),(200,40),(450,40),(893,40),
    (0,60),(10,60),(100,60),(159,60),(160,60),(170,60),(179,60),(180,60),(500,60),(879,60),(880,60),(893,60),
    (10,100),(50,100),(100,100),(159,100),(180,100),(500,100),(879,100),
    (10,300),(50,300),(100,300),(159,300),(180,300),(500,300),(879,300),
    (10,450),(50,450),(100,450),(159,450),(180,450),(500,450),(879,450),
    (10,640),(100,640),(159,640),(180,640),(500,640),(879,640),
    (893,655),(0,655),(450,655),
]
print("--- sample colors (x,y) -> rgb ---")
for x,y in pts:
    print(f"  ({x:3d},{y:3d}) = {tuple(int(v) for v in a[y,x])}")

# ---------- dominant color per horizontal band ----------
print("--- dominant colors by 8px band ---")
for y0 in range(0, H, 8):
    band = a[y0:min(y0+8,H)]
    c = Counter(map(tuple, band.reshape(-1,3)))
    top = c.most_common(3)
    print(f"  y={y0:3d}: {top}")

# ---------- strong uniform horizontal lines ----------
print("--- uniform horizontal lines (run>=30, same rgb) ---")
for y in range(H):
    row = a[y]
    # find runs of identical rgb
    runs = []
    s = 0
    for x in range(1, W):
        if not np.array_equal(row[x], row[s]):
            if x - s >= 30:
                runs.append((s, x-1, tuple(int(v) for v in row[s])))
            s = x
    if W - s >= 30:
        runs.append((s, W-1, tuple(int(v) for v in row[s])))
    if runs:
        print(f"  y={y:3d}: {runs[:6]}")

# ---------- strong uniform vertical lines ----------
print("--- uniform vertical lines (run>=30, same rgb) ---")
for x in range(W):
    col = a[:, x]
    runs = []
    s = 0
    for y in range(1, H):
        if not np.array_equal(col[y], col[s]):
            if y - s >= 30:
                runs.append((s, y-1, tuple(int(v) for v in col[s])))
            s = y
    if H - s >= 30:
        runs.append((s, H-1, tuple(int(v) for v in col[s])))
    if runs:
        print(f"  x={x:3d}: {runs[:6]}")
