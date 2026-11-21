import sys
from winrt.windows.storage import StorageFile, FileAccessMode
from winrt.windows.graphics.imaging import BitmapDecoder
from winrt.windows.media.ocr import OcrEngine
from winrt.windows.globalization import Language

path = sys.argv[1]
file = StorageFile.get_file_from_path_async(path).get()
stream = file.open_async(FileAccessMode.READ).get()
decoder = BitmapDecoder.create_async(stream).get()
bitmap = decoder.get_software_bitmap_async().get()
engine = OcrEngine.try_create_from_language(Language("ru"))
result = engine.recognize_async(bitmap).get()

lines = []
for line in result.lines:
    words = []
    for w in line.words:
        r = w.bounding_rect
        words.append((r.x, r.y, r.width, r.height, w.text))
    if words:
        x0 = min(w[0] for w in words); y0 = min(w[1] for w in words)
        x1 = max(w[0]+w[2] for w in words); y1 = max(w[1]+w[3] for w in words)
    else:
        x0=y0=x1=y1=0
    lines.append((x0, y0, x1-x0, y1-y0, line.text, words))

lines.sort(key=lambda l: (l[1], l[0]))
with open("D:/2026RustLev/RustFold/analysis/ocr_layout.txt", "w", encoding="utf-8") as f:
    for x, y, w, h, text, words in lines:
        f.write("[%4d,%4d %4dx%3d] %s\n" % (x, y, w, h, text))
        for wx, wy, ww, wh, wt in words:
            f.write("        [%4d,%4d %4dx%3d] %s\n" % (wx, wy, ww, wh, wt))
print("lines:", len(lines))
