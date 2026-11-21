import ctypes, ctypes.wintypes as w, time, sys, os

EXE = r"C:\Users\neon\Downloads\03_Fold\03_Fold.exe"
OUT = r"D:\2026RustLev\RustFold\analysis\memory"
os.makedirs(OUT, exist_ok=True)

k32 = ctypes.WinDLL("kernel32", use_last_error=True)

class STARTUPINFO(ctypes.Structure):
    _fields_ = [
        ("cb", w.DWORD), ("lpReserved", w.LPSTR), ("lpDesktop", w.LPSTR),
        ("lpTitle", w.LPSTR), ("dwX", w.DWORD), ("dwY", w.DWORD),
        ("dwXSize", w.DWORD), ("dwYSize", w.DWORD), ("dwXCountChars", w.DWORD),
        ("dwYCountChars", w.DWORD), ("dwFillAttribute", w.DWORD),
        ("dwFlags", w.DWORD), ("wShowWindow", w.WORD), ("cbReserved2", w.WORD),
        ("lpReserved2", ctypes.POINTER(ctypes.c_byte)), ("hStdInput", w.HANDLE),
        ("hStdOutput", w.HANDLE), ("hStdError", w.HANDLE),
    ]

class PROCESS_INFORMATION(ctypes.Structure):
    _fields_ = [("hProcess", w.HANDLE), ("hThread", w.HANDLE),
                ("dwProcessId", w.DWORD), ("dwThreadId", w.DWORD)]
CREATE_SUSPENDED = 0x00000004
CREATE_NO_WINDOW = 0x08000000
PROCESS_VM_READ = 0x0010
PROCESS_TERMINATE = 0x0001
PAGE_SIZE = 0x1000

k32.CreateProcessA.argtypes = [
    w.LPCSTR, w.LPSTR, w.LPVOID, w.LPVOID, w.BOOL, w.DWORD,
    w.LPVOID, w.LPCSTR, ctypes.POINTER(STARTUPINFO), ctypes.POINTER(PROCESS_INFORMATION)
]
k32.CreateProcessA.restype = w.BOOL
k32.ReadProcessMemory.argtypes = [w.HANDLE, w.LPVOID, w.LPVOID, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]
k32.ReadProcessMemory.restype = w.BOOL
k32.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
k32.OpenProcess.restype = w.HANDLE

si = STARTUPINFO()
si.cb = ctypes.sizeof(si)
pi = PROCESS_INFORMATION()

cmd = ctypes.create_string_buffer(('"' + EXE + '"').encode("ascii"))
ok = k32.CreateProcessA(None, cmd, None, None, False,
                        CREATE_SUSPENDED | CREATE_NO_WINDOW, None, None,
                        ctypes.byref(si), ctypes.byref(pi))
if not ok:
    print("CreateProcess failed:", ctypes.get_last_error())
    sys.exit(1)

print("PID", pi.dwProcessId, "TID", pi.dwThreadId)
print("image base from PE = 0x400000, SizeOfImage = 0xA8000")

# Let the ASPack stub unpack and the app initialize
k32.ResumeThread(pi.hThread)
time.sleep(2.5)

h = k32.OpenProcess(PROCESS_VM_READ | PROCESS_TERMINATE, False, pi.dwProcessId)
if not h:
    print("OpenProcess failed:", ctypes.get_last_error())
    sys.exit(1)

base = 0x400000
size = 0xA8000
buf = bytearray(size)
read_ok = 0
for off in range(0, size, PAGE_SIZE):
    chunk = (ctypes.c_ubyte * min(PAGE_SIZE, size - off))()
    got = ctypes.c_size_t(0)
    if k32.ReadProcessMemory(h, ctypes.c_void_p(base + off), chunk, len(chunk), ctypes.byref(got)):
        buf[off:off+got.value] = bytes(chunk[:got.value])
        read_ok += 1

dump_path = os.path.join(OUT, "unpacked_image.bin")
open(dump_path, "wb").write(bytes(buf))
print("dumped %d/%d pages -> %s" % (read_ok, size // PAGE_SIZE, dump_path))

k32.TerminateProcess(h, 0)
k32.CloseHandle(h)
k32.CloseHandle(pi.hThread)
k32.CloseHandle(pi.hProcess)
