use std::io::Write;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GetWindowRect,
};

macro_rules! log {
    ($($arg:tt)*) => {{
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open("D:/desktop_cleaner.log")
        {
            let _ = writeln!(f, $($arg)*);
        }
    }};
}

// ── Win32 原生 FFI ──
extern "system" {
    fn GetWindowLongPtrW(hWnd: isize, nIndex: i32) -> isize;
    fn SetWindowLongPtrW(hWnd: isize, nIndex: i32, dwNewLong: isize) -> isize;
    fn SendMessageW(hWnd: isize, Msg: u32, wParam: usize, lParam: isize) -> isize;
    fn InvalidateRect(hWnd: isize, lpRect: *const RECT, bErase: i32) -> i32;
}

const LVM_FIRST: u32 = 0x1000;
const LVM_GETITEMCOUNT: u32 = LVM_FIRST + 4;
const LVM_SETITEMPOSITION: u32 = LVM_FIRST + 15;
const LVM_ARRANGE: u32 = LVM_FIRST + 22;
const GWL_STYLE: i32 = -16;
const LVS_AUTOARRANGE: isize = 0x100;

fn find_desktop_listview() -> Option<HWND> {
    unsafe {
        if let Ok(progman) = FindWindowW(w!("Progman"), None) {
            if !progman.0.is_null() {
                if let Ok(def_view) =
                    FindWindowExW(progman, None, w!("SHELLDLL_DefView"), None)
                {
                    if !def_view.0.is_null() {
                        if let Ok(lv) =
                            FindWindowExW(def_view, None, w!("SysListView32"), None)
                        {
                            if !lv.0.is_null() {
                                log!("[ok] Progman->DefView->SysListView32, hwnd={:?}", lv.0);
                                return Some(lv);
                            }
                        }
                    }
                }
            }
        }
        let mut worker = HWND(null_mut());
        loop {
            worker = match FindWindowExW(None, worker, w!("WorkerW"), None) {
                Ok(h) => h,
                Err(_) => break,
            };
            if worker.0.is_null() { break; }
            if let Ok(def_view) =
                FindWindowExW(worker, None, w!("SHELLDLL_DefView"), None)
            {
                if !def_view.0.is_null() {
                    if let Ok(lv) =
                        FindWindowExW(def_view, None, w!("SysListView32"), None)
                    {
                        if !lv.0.is_null() {
                            log!("[ok] WorkerW->DefView->SysListView32, hwnd={:?}", lv.0);
                            return Some(lv);
                        }
                    }
                }
            }
        }
        log!("[err] 未找到桌面 SysListView32");
        None
    }
}

fn lv_disable_auto_arrange(lv: HWND) {
    unsafe {
        let h = lv.0 as isize;
        let style = GetWindowLongPtrW(h, GWL_STYLE);
        log!("[info] ListView style=0x{:X}, has_auto={}", style, style & LVS_AUTOARRANGE != 0);
        if style & LVS_AUTOARRANGE != 0 {
            SetWindowLongPtrW(h, GWL_STYLE, style & !LVS_AUTOARRANGE);
            log!("[ok] 已禁用 LVS_AUTOARRANGE");
        } else {
            log!("[info] LVS_AUTOARRANGE 原本就未设置");
        }
    }
}

fn lv_enable_auto_arrange(lv: HWND) {
    unsafe {
        let h = lv.0 as isize;
        let style = GetWindowLongPtrW(h, GWL_STYLE);
        SetWindowLongPtrW(h, GWL_STYLE, style | LVS_AUTOARRANGE);
        SendMessageW(h, LVM_ARRANGE, 0, 0);
        InvalidateRect(h, std::ptr::null(), 1);
        log!("[ok] 已恢复 LVS_AUTOARRANGE + LVM_ARRANGE");
    }
}

fn lv_item_count(lv: HWND) -> usize {
    unsafe {
        let count = SendMessageW(lv.0 as isize, LVM_GETITEMCOUNT, 0, 0) as usize;
        log!("[info] 桌面图标数量: {}", count);
        count
    }
}

fn pack_xy(x: i32, y: i32) -> isize {
    // LVM_SETITEMPOSITION: lParam = MAKELPARAM(x, y)
    // x 在低 16 位，y 在高 16 位（有符号扩展需注意）
    ((x & 0xFFFF) as isize) | (((y & 0xFFFF) as isize) << 16)
}

fn get_our_window_rect() -> Option<RECT> {
    unsafe {
        let hwnd = match FindWindowW(None, w!("桌面整理大师")) {
            Ok(h) => h,
            Err(_) => { log!("[err] FindWindowW 失败"); return None; }
        };
        if hwnd.0.is_null() { log!("[err] 窗口 hwnd 为空"); return None; }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            log!("[err] GetWindowRect 失败");
            return None;
        }
        log!("[info] 程序窗口: L{} T{} R{} B{}", rect.left, rect.top, rect.right, rect.bottom);
        Some(rect)
    }
}

// ── 核心：移动图标到窗口下方 ──

fn reposition_icons_at(rect: RECT) {
    let Some(lv) = find_desktop_listview() else { return };
    lv_disable_auto_arrange(lv);

    let count = lv_item_count(lv);
    if count == 0 { return; }

    // 图标随机散落在整个窗口区域内，被程序界面遮挡
    let area_w = rect.right - rect.left - 70;
    let area_h = rect.bottom - rect.top - 70;
    let area_w = if area_w < 50 { 50 } else { area_w };
    let area_h = if area_h < 50 { 50 } else { area_h };

    log!("[info] 移动 {} 个图标到窗口区域 {}x{}", count, area_w, area_h);

    for i in 0..count {
        // 乘法哈希生成伪随机坐标
        let s = (i as u32).wrapping_mul(0x9E3779B1);
        let x = rect.left + 10 + (s % area_w as u32) as i32;
        let s2 = s.wrapping_mul(0x85EBCA77);
        let y = rect.top + 10 + (s2 % area_h as u32) as i32;
        let lp = pack_xy(x, y);
        unsafe {
            SendMessageW(lv.0 as isize, LVM_SETITEMPOSITION, i, lp);
        }
    }

    unsafe {
        InvalidateRect(lv.0 as isize, std::ptr::null(), 1);
    }
    log!("[ok] {} 个图标已移动", count);
}

// ── 公开接口 ──

pub fn stack_icons_under_window() {
    log!("=== stack_icons_under_window ===");
    if let Some(rect) = get_our_window_rect() {
        reposition_icons_at(rect);
    }
}

pub fn restore_desktop() {
    log!("=== restore_desktop ===");
    if let Some(lv) = find_desktop_listview() {
        lv_enable_auto_arrange(lv);
    }
}

pub fn start_drag_monitor(active: Arc<AtomicBool>) {
    log!("=== start_drag_monitor ===");
    thread::spawn(move || {
        let mut last_pos: Option<(i32, i32)> = None;
        let mut has_moved = false;
        let mut stable_since: Option<(Instant, RECT)> = None;
        let mut cached_hwnd: Option<HWND> = None;

        loop {
            if !active.load(Ordering::Relaxed) { break; }
            if cached_hwnd.map_or(true, |h| h.0.is_null()) {
                unsafe {
                    if let Ok(hwnd) = FindWindowW(None, w!("桌面整理大师")) {
                        if !hwnd.0.is_null() { cached_hwnd = Some(hwnd); }
                    }
                }
            }
            if let Some(hwnd) = cached_hwnd {
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
                    let pos = (rect.left, rect.top);
                    if Some(pos) != last_pos {
                        last_pos = Some(pos);
                        stable_since = None;
                        if !has_moved { log!("[monitor] 检测到窗口移动"); }
                        has_moved = true;
                    } else if has_moved && stable_since.is_none() {
                        stable_since = Some((Instant::now(), rect));
                    }
                    if let Some((since, stable_rect)) = stable_since {
                        if since.elapsed() >= Duration::from_millis(500) {
                            log!("[monitor] 静止 500 毫秒，重新堆叠到 ({},{})", stable_rect.left, stable_rect.top);
                            reposition_icons_at(stable_rect);
                            stable_since = None;
                            has_moved = false;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(150));
        }
        log!("[monitor] 退出");
    });
}
