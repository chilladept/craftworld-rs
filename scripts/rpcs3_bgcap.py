"""Capture the RPCS3 game window without focusing it (PrintWindow, full content). usage: bgcap.py out.png [width]"""
import sys, os, ctypes, ctypes.wintypes as wt
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'local-analysis', 'pc-helpers'))
import pad
from PIL import Image
u, g = pad.u32, ctypes.WinDLL('gdi32')
h = pad.game_window()
r = wt.RECT(); u.GetClientRect(h, ctypes.byref(r)); w, hh = r.right, r.bottom
dc = u.GetDC(h); mdc = g.CreateCompatibleDC(dc); bmp = g.CreateCompatibleBitmap(dc, w, hh); g.SelectObject(mdc, bmp)
u.PrintWindow(h, mdc, 3)
class BIH(ctypes.Structure):
    _fields_ = [('s', wt.DWORD), ('w', wt.LONG), ('h', wt.LONG), ('p', wt.WORD), ('b', wt.WORD), ('c', wt.DWORD), ('si', wt.DWORD), ('x', wt.LONG), ('y', wt.LONG), ('u', wt.DWORD), ('i', wt.DWORD)]
bi = BIH(40, w, -hh, 1, 32, 0, 0, 0, 0, 0, 0)
buf = ctypes.create_string_buffer(w * hh * 4)
g.GetDIBits(mdc, bmp, 0, hh, buf, ctypes.byref(bi), 0)
im = Image.frombuffer('RGBA', (w, hh), buf, 'raw', 'BGRA', 0, 1).convert('RGB')
W = int(sys.argv[2]) if len(sys.argv) > 2 else 480
im.resize((W, int(hh * W / w))).save(sys.argv[1]); print('ok', w, hh)
