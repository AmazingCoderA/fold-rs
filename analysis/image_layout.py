import cv2, numpy as np, sys
from collections import Counter

path = sys.argv[1]
im = cv2.imread(path)
h, w = im.shape[:2]
print("size", w, h)

#_top colors
small = cv2.resize(im, (w//4, h//4), interpolation=cv2.INTER_AREA)
cnt = Counter(map(tuple, small.reshape(-1,3)))
print("top colors:")
for c, n in cnt.most_common(16):
    print("  BGR", c, n)

gray = cv2.cvtColor(im, cv2.COLOR_BGR2GRAY)
# long horizontal/vertical edges
edges = cv2.Canny(gray, 50, 150)
lines = cv2.HoughLinesP(edges, 1, np.pi/180, threshold=120, minLineLength=100, maxLineGap=4)
print("long lines:")
res=[]
if lines is not None:
    for l in lines[:,0]:
        x1,y1,x2,y2 = map(int,l)
        if abs(y2-y1)<=2 or abs(x2-x1)<=2:
            res.append((x1,y1,x2,y2))
for x1,y1,x2,y2 in sorted(res, key=lambda q:(q[1],q[0]))[:80]:
    print("  ",x1,y1,x2,y2)

# column/row darkness profile to find panel borders
col = (255-gray).mean(axis=0)
row = (255-gray).mean(axis=1)
print("dark columns (peaks):", sorted(range(w), key=lambda i:col[i], reverse=True)[:20])
print("dark rows (peaks):", sorted(range(h), key=lambda i:row[i], reverse=True)[:20])

# upscale for OCR
up = cv2.resize(im, None, fx=3, fy=3, interpolation=cv2.INTER_CUBIC)
cv2.imwrite("D:/2026RustLev/RustFold/analysis/screen_up.png", up)
print("saved up", up.shape)
