"""Background key presses: PostMessage to the RPCS3 game window, never foreground it.
usage: rpcs3_bgpad.py KEY MS   (needs local-analysis/pc-helpers/pad.py for the VK table and window lookup)"""
import ctypes, ctypes.wintypes as wt, time, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'local-analysis', 'pc-helpers'))
import pad
u32 = pad.u32
WM_KEYDOWN, WM_KEYUP = 0x0100, 0x0101
def _lp(vk, up):
    scan = u32.MapVirtualKeyW(vk, 0)
    ext = 1 if 0x21 <= vk <= 0x28 else 0
    lp = 1 | (scan << 16) | (ext << 24)
    if up: lp |= (1 << 30) | (1 << 31)
    return lp
def hold(k, ms):
    h = pad.game_window()
    if not h: raise RuntimeError('game window not found')
    vk = pad.VK[k]
    u32.PostMessageW(h, WM_KEYDOWN, vk, _lp(vk, False))
    time.sleep(ms / 1000)
    u32.PostMessageW(h, WM_KEYUP, vk, _lp(vk, True))
    time.sleep(0.05)

if __name__ == '__main__':
    hold(sys.argv[1], int(sys.argv[2]))
