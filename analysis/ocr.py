import sys
from winrt.windows.storage import StorageFile
from winrt.windows.storage import FileAccessMode
from winrt.windows.graphics.imaging import BitmapDecoder
from winrt.windows.media.ocr import OcrEngine
from winrt.windows.globalization import Language

path = sys.argv[1]
file = StorageFile.get_file_from_path_async(path).get()
stream = file.open_async(FileAccessMode.READ).get()
decoder = BitmapDecoder.create_async(stream).get()
bitmap = decoder.get_software_bitmap_async().get()

for tag in ("ru", "en-US"):
    engine = OcrEngine.try_create_from_language(Language(tag))
    if engine is None:
        print("NO ENGINE", tag)
        continue
    print("=== ENGINE", tag, "===")
    result = engine.recognize_async(bitmap).get()
    for line in result.lines:
        t = line.text
        print("TEXT", t.encode("unicode_escape").decode("ascii"))
        for enc in ("cp1251", "cp866", "latin1"):
            try:
                print(enc, t.encode("latin1").decode(enc))
            except Exception as e:
                print(enc, "ERR", e)
    break
