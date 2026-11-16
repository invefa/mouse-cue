// mouse-cue: 鼠标操作预约器。
// 读一份计划文件，倒计时期间在屏幕顶部显示提示，倒计时结束后按顺序执行
// 移动、点击、等待，并在目标位置画出圆环特效。用户可按 Esc 或点浮层上的
// 取消区中止，程序以退出码 2 收尾并输出「操作失败: 用户取消」。

#![allow(non_snake_case, non_upper_case_globals)]

mod dcomp;

use std::env;
use std::fs;
use std::time::{Duration, Instant};

type HWND = isize;
type HINSTANCE = isize;
type HBRUSH = isize;
type HPEN = isize;
type HFONT = isize;
type HDC = isize;

#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: POINT,
    private: u32,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: HDC,
    erase: i32,
    rc: RECT,
    restore: i32,
    inc_update: i32,
    reserved: [u8; 32],
}

#[repr(C)]
struct WNDCLASSW {
    style: u32,
    wnd_proc: usize,
    cls_extra: i32,
    wnd_extra: i32,
    instance: HINSTANCE,
    icon: isize,
    cursor: isize,
    background: HBRUSH,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct MOUSEINPUT {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
struct INPUT {
    kind: u32,
    pad: u32,
    mi: MOUSEINPUT,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
}


#[repr(C)]
struct GdiplusStartupInput {
    version: u32,
    debug_callback: usize,
    suppress_background: i32,
    suppress_codecs: i32,
}

#[link(name = "gdiplus")]
extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut u16) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipCreateFromHDC(hdc: HDC, graphics: *mut isize) -> i32;
    fn GdipDeleteGraphics(graphics: isize) -> i32;
    fn GdipSetSmoothingMode(graphics: isize, mode: i32) -> i32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut isize) -> i32;
    fn GdipDeletePen(pen: isize) -> i32;
    fn GdipDrawEllipse(graphics: isize, pen: isize, x: f32, y: f32, w: f32, h: f32) -> i32;
    fn GdipDrawLine(graphics: isize, pen: isize, x1: f32, y1: f32, x2: f32, y2: f32) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut isize) -> i32;
    fn GdipDeleteBrush(brush: isize) -> i32;
    fn GdipFillEllipse(graphics: isize, brush: isize, x: f32, y: f32, w: f32, h: f32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: HDC) -> HDC;
    fn CreateCompatibleBitmap(hdc: HDC, w: i32, h: i32) -> isize;
    fn BitBlt(dst: HDC, x: i32, y: i32, w: i32, h: i32, src: HDC, sx: i32, sy: i32, rop: u32) -> i32;
    fn DeleteDC(hdc: HDC) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(class: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: isize,
        instance: HINSTANCE,
        param: *mut std::ffi::c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize;
    fn ShowWindow(hwnd: HWND, cmd: i32) -> i32;
    fn UpdateWindow(hwnd: HWND) -> i32;
    fn SetTimer(hwnd: HWND, id: usize, ms: u32, callback: usize) -> usize;
    fn KillTimer(hwnd: HWND, id: usize) -> i32;
    fn GetMessageW(msg: *mut MSG, hwnd: HWND, min: u32, max: u32) -> i32;
    fn TranslateMessage(msg: *const MSG) -> i32;
    fn DispatchMessageW(msg: *const MSG) -> isize;
    fn PostQuitMessage(code: i32);
    fn DestroyWindow(hwnd: HWND) -> i32;
    fn SetWindowPos(hwnd: HWND, after: HWND, x: i32, y: i32, w: i32, h: i32, flags: u32) -> i32;
    fn SetLayeredWindowAttributes(hwnd: HWND, key: u32, alpha: u8, flags: u32) -> i32;
    fn GetSystemMetrics(index: i32) -> i32;
    fn GetDpiForSystem() -> u32;
    fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    fn LoadCursorW(instance: HINSTANCE, name: *const u16) -> isize;
    fn BeginPaint(hwnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    fn EndPaint(hwnd: HWND, paint: *const PAINTSTRUCT) -> i32;
    fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> i32;
    fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    fn GetAsyncKeyState(key: i32) -> i16;
    fn SetCursorPos(x: i32, y: i32) -> i32;
    fn SendInput(count: u32, inputs: *const INPUT, size: i32) -> u32;
    fn GetDC(hwnd: HWND) -> HDC;
    fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    fn IsWindowVisible(hwnd: HWND) -> i32;
    fn SetWindowRgn(hwnd: HWND, rgn: isize, redraw: i32) -> i32;
    fn SetProcessDPIAware() -> i32;
    fn GetCursorPos(pt: *mut POINT) -> i32;
    fn mouse_event(flags: u32, dx: i32, dy: i32, data: u32, extra: usize);
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateSolidBrush(color: u32) -> HBRUSH;
    fn CreatePen(style: u32, width: i32, color: u32) -> HPEN;
    fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike: u32,
        charset: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch: u32,
        face: *const u16,
    ) -> HFONT;
    fn SelectObject(hdc: HDC, obj: isize) -> isize;
    fn DeleteObject(obj: isize) -> i32;
    fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    fn SetTextColor(hdc: HDC, color: u32) -> u32;
    fn DrawTextW(hdc: HDC, text: *const u16, len: i32, rect: *mut RECT, format: u32) -> i32;
    fn Ellipse(hdc: HDC, l: i32, t: i32, r: i32, b: i32) -> i32;
    fn MoveToEx(hdc: HDC, x: i32, y: i32, prev: *mut POINT) -> i32;
    fn LineTo(hdc: HDC, x: i32, y: i32) -> i32;
    fn RoundRect(hdc: HDC, l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> i32;
    fn GetStockObject(index: i32) -> isize;
    fn GetPixel(hdc: HDC, x: i32, y: i32) -> u32;
    fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> i32;
    fn CreateRoundRectRgn(l: i32, t: i32, r: i32, b: i32, w: i32, h: i32) -> isize;
    fn CreateEllipticRgn(l: i32, t: i32, r: i32, b: i32) -> isize;
}

const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_TOPMOST: u32 = 0x0000_0008;
const WS_EX_LAYERED: u32 = 0x0008_0000;
const WS_EX_TRANSPARENT: u32 = 0x0000_0020;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;

const SW_SHOWNOACTIVATE: i32 = 4;
const SWP_NOACTIVATE: u32 = 0x0010;
const SWP_SHOWWINDOW: u32 = 0x0040;
const HWND_TOPMOST: HWND = -1;

const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_TIMER: u32 = 0x0113;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_NCHITTEST: u32 = 0x0084;

const DT_LEFT: u32 = 0;
const DT_RIGHT: u32 = 0x0000_0002;
const DT_VCENTER: u32 = 0x0000_0004;
const DT_SINGLELINE: u32 = 0x0000_0020;
const DT_NOPREFIX: u32 = 0x0000_0800;

const TRANSPARENT_BK: i32 = 1;
const VK_ESCAPE: i32 = 0x1B;

const MOVE: u32 = 0x0001;
const LEFT_DOWN: u32 = 0x0002;
const LEFT_UP: u32 = 0x0004;
const ABSOLUTE: u32 = 0x8000;
const VIRTUAL_DESK: u32 = 0x4000;

/* palette: greyscale only. hierarchy comes from brightness and line weight,
 * so the countdown and the cue ring use the brightest tone instead of a hue. */
const CLR_BG: u32 = 0x001E_1E1E; // #1e1e1e
const CLR_EDGE: u32 = 0x0036_3636; // #363636
const CLR_TEXT: u32 = 0x00E8_E8E8; // #e8e8e8
const CLR_MUTED: u32 = 0x008C_8C8C; // #8c8c8c
const CLR_ACCENT: u32 = 0x00DC_DCDC; // #dcdcdc
const CLR_STOP: u32 = 0x00FF_FFFF; // #ffffff
// GDI+ 的颜色是 ARGB，alpha 位必须给满，否则整块画不出来
const A_DISC: u32 = 0xFF3E_3E3E;
const A_DISC_HOT: u32 = 0xFFA8_A8A8;
const A_RIM: u32 = 0xFFF2_F2F2;
const A_SHADOW: u32 = 0xFF18_1818;
const A_CROSS: u32 = 0xFFF8_F8F8;

#[derive(Clone)]
enum Step {
    Move { x: i32, y: i32, label: String },
    Click { double: bool },
    Wait { ms: u64 },
    Exec { line: String },
}

struct Plan {
    title: String,
    detail: String,
    countdown: u64,
    steps: Vec<Step>,
}

struct State {
    plan: Plan,
    cancel_at: Option<Instant>,
    phase: u8, // 0 countdown, 1 running, 2 done, 3 cancelled
    countdown_until: Instant,
    step: usize,
    wait_until: Option<Instant>,
    done_until: Instant,
    started: Instant,
    main: HWND,
    cue: HWND,
    cue_at: (i32, i32),
    cue_pressed: bool,
    cue_from: f32,
    cue_to: f32,
    cue_started: Instant,
    cue_ms: f32,
    cue_radius: f32,
    disc_color: u32,
    cross_color: u32,
    cancel_rect: RECT,
    dpi: f32,
    exit_code: i32,
    last_label: String,
    surface: Option<dcomp::MarkerSurface>,
    last_push: Option<(f32, f32, f32, f32, bool)>,
}

static mut STATE: Option<State> = None;

fn wide(text: &str) -> Vec<u16> {
    let mut out: Vec<u16> = text.encode_utf16().collect();
    out.push(0);
    out
}

fn st() -> &'static mut State {
    unsafe { STATE.as_mut().unwrap() }
}

fn scale(dpi: f32, value: i32) -> i32 {
    (value as f32 * dpi).round() as i32
}

pub(crate) fn dpi_scale() -> f32 {
    unsafe { (GetDpiForSystem() as f32) / 96.0 }
}

pub(crate) fn screen_size() -> (i32, i32) {
    unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) }
}

fn parse_plan(text: &str) -> Result<Plan, String> {
    let mut plan = Plan {
        title: String::from("鼠标操作"),
        detail: String::new(),
        countdown: 3,
        steps: Vec::new(),
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, rest) = match line.find('=') {
            Some(pos) => (&line[..pos], line[pos + 1..].trim()),
            None => (line, ""),
        };
        match key.trim() {
            "title" => plan.title = rest.to_string(),
            "detail" => plan.detail = rest.to_string(),
            "countdown" => {
                plan.countdown = rest
                    .parse::<u64>()
                    .map_err(|_| format!("第 {} 行: countdown 需要整数秒", index + 1))?
                    .clamp(0, 120);
            }
            "move" => {
                let parts: Vec<&str> = rest.splitn(3, ',').collect();
                if parts.len() < 2 {
                    return Err(format!("第 {} 行: move 需要 x,y[,标签]", index + 1));
                }
                let x = parts[0]
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| format!("第 {} 行: x 不是整数", index + 1))?;
                let y = parts[1]
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| format!("第 {} 行: y 不是整数", index + 1))?;
                let label = parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
                plan.steps.push(Step::Move { x, y, label });
            }
            "exec" => {
                if rest.is_empty() {
                    return Err(format!("第 {} 行: exec 需要命令行", index + 1));
                }
                plan.steps.push(Step::Exec { line: rest.to_string() });
            }
            "click" => plan.steps.push(Step::Click { double: false }),
            "double" => plan.steps.push(Step::Click { double: true }),
            "wait" => {
                let ms = rest
                    .parse::<u64>()
                    .map_err(|_| format!("第 {} 行: wait 需要毫秒", index + 1))?
                    .min(60_000);
                plan.steps.push(Step::Wait { ms });
            }
            other => return Err(format!("第 {} 行: 不认识的动作 {}", index + 1, other)),
        }
    }
    if plan.steps.is_empty() {
        return Err(String::from("计划里没有动作"));
    }
    Ok(plan)
}

fn cursor_at() -> (i32, i32) {
    let mut at = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut at);
    }
    (at.x, at.y)
}

/// 定位到目标。SetCursorPos 与 SendInput 绝对移动在不同宿主上各有失灵的时候，
/// 两者交替重试三次，最后返回是否到位。
fn move_cursor(x: i32, y: i32) -> bool {
    let (w, h) = screen_size();
    let nx = ((x as f64) * 65535.0 / ((w - 1).max(1) as f64)).round() as i32;
    let ny = ((y as f64) * 65535.0 / ((h - 1).max(1) as f64)).round() as i32;
    for attempt in 0..2 {
        let (cx, cy) = cursor_at();
        if (cx, cy) == (x, y) {
            return true;
        }
        match attempt % 3 {
            // 相对移动不依赖屏幕尺寸与 DPI 换算，作为主路径
            0 | 1 => send_mouse(x - cx, y - cy, MOVE),
            // 绝对移动与直接定位作为备选
            2 => send_mouse(nx, ny, MOVE | ABSOLUTE),
            _ => unsafe {
                SetCursorPos(x, y);
            },
        }
        std::thread::sleep(Duration::from_millis(20));
        eprintln!("move attempt {} want=({},{}) at={:?}", attempt + 1, x, y, cursor_at());
    }
    cursor_at() == (x, y)
}

fn normalized(x: i32, y: i32) -> (i32, i32) {
    let (w, h) = screen_size();
    let nx = ((x as f32) * 65535.0 / ((w - 1).max(1) as f32)).round() as i32;
    let ny = ((y as f32) * 65535.0 / ((h - 1).max(1) as f32)).round() as i32;
    (nx, ny)
}

fn send_mouse(dx: i32, dy: i32, flags: u32) {
    let input = INPUT {
        kind: 0,
        pad: 0,
        mi: MOUSEINPUT {
            dx,
            dy,
            mouse_data: 0,
            flags,
            time: 0,
            extra: 0,
        },
    };
    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
}

fn absolute_move(x: i32, y: i32) {
    unsafe {
        SetCursorPos(x, y);
    }
}

// 真实化点击：先落位，插入两次 1px 抖动，再按下并停留，最后抬起。
// React 这类合成事件层对「纯 down/up」不敏感，抖动可让事件序列更像真人输入。
fn do_click(x: i32, y: i32, double: bool) {
    absolute_move(x, y);
    std::thread::sleep(Duration::from_millis(16));
    absolute_move(x + 1, y);
    std::thread::sleep(Duration::from_millis(16));
    absolute_move(x, y + 1);
    std::thread::sleep(Duration::from_millis(16));
    absolute_move(x, y);
    std::thread::sleep(Duration::from_millis(24));
    let rounds = if double { 2 } else { 1 };
    for _ in 0..rounds {
        // mouse_event 在 Electron 宿主上比 SendInput 更稳定地产生完整事件链
        unsafe {
            mouse_event(LEFT_DOWN, 0, 0, 0, 0);
        }
        std::thread::sleep(Duration::from_millis(80));
        unsafe {
            mouse_event(LEFT_UP, 0, 0, 0, 0);
        }
        if double {
            std::thread::sleep(Duration::from_millis(60));
        }
    }
}

/// 采样目标位置的底色，挑一组对比最强的配色：暗底给亮图标，亮底给暗图标。
fn pick_colors(x: i32, y: i32) -> (u32, u32) {
    unsafe {
        let dc = GetDC(0);
        if dc == 0 {
            return (0xFF3C_3C3C, 0xFFF4_F4F4);
        }
        let samples = [(0, 0), (-7, 0), (7, 0), (0, -7), (0, 7), (-5, -5), (5, 5)];
        let mut total = 0u32;
        let mut count = 0u32;
        for (dx, dy) in samples {
            let px = GetPixel(dc, x + dx, y + dy);
            if px == 0xFFFF_FFFF {
                continue;
            }
            let r = px & 0xFF;
            let g = (px >> 8) & 0xFF;
            let b = (px >> 16) & 0xFF;
            total += (r * 299 + g * 587 + b * 114) / 1000;
            count += 1;
        }
        ReleaseDC(0, dc);
        let lum = if count > 0 { total / count } else { 128 };
        if lum < 120 {
            (0xFF3C_3C3C, 0xFFF4_F4F4)
        } else {
            (0xFFC6_C6C6, 0xFF14_1414)
        }
    }
}

/// 把落点标记放到目标处。animate 为真时从三倍尺寸收缩到定尺。
fn place_cue(x: i32, y: i32, animate: bool) {
    let state = st();
    // 标记窗口固定铺满整屏，原点在 (1,1)，窗口内坐标即屏幕坐标减一
    let (sw, _sh) = screen_size();
    let start = sw as f32 / 2.0;
    let end = scale(state.dpi, 40) as f32;
    state.cue_at = (x - 1, y - 1);
    state.cue_to = end;
    state.cue_from = if animate { start } else { end };
    state.cue_radius = state.cue_from;
    state.cue_started = Instant::now();
    unsafe {
        ShowWindow(state.cue, SW_SHOWNOACTIVATE);
    }
    push_surface(state);
}

/// 推进收缩动画；返回是否仍在动。
fn step_cue_animation() -> bool {
    let state = st();
    let elapsed = Instant::now().saturating_duration_since(state.cue_started).as_secs_f32() * 1000.0;
    if elapsed >= state.cue_ms {
        if (state.cue_radius - state.cue_to).abs() > 0.01 {
            state.cue_radius = state.cue_to;
            invalidate(state.cue);
        }
        return false;
    }
    let t = (elapsed / state.cue_ms).clamp(0.0, 1.0);
    let ease = 1.0 - (1.0 - t).powi(3);
    let next = state.cue_from + (state.cue_to - state.cue_from) * ease;
    if (next - state.cue_radius).abs() > 0.4 {
        state.cue_radius = next;
        invalidate(state.cue);
    }
    true
}

fn invalidate(hwnd: HWND) {
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
}

fn draw_main(hwnd: HWND, hdc: HDC) {
    let state = st();
    let dpi = state.dpi;
    unsafe {
        let mut client = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd, &mut client);

        let panel = CreateSolidBrush(CLR_BG);
        FillRect(hdc, &client, panel);
        DeleteObject(panel);

        let border = CreatePen(0, 1, CLR_EDGE);
        let hollow = CreateSolidBrush(CLR_BG);
        let old_brush = SelectObject(hdc, hollow);
        let old_pen = SelectObject(hdc, border);
        let radius = scale(dpi, 10);
        RoundRect(
            hdc,
            0,
            0,
            client.right,
            client.bottom,
            radius,
            radius,
        );
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(border);
        DeleteObject(hollow);

        SetBkMode(hdc, TRANSPARENT_BK);

        let face = wide("Microsoft YaHei");
        let title_font = CreateFontW(
            -scale(dpi, 16),
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );
        let body_font = CreateFontW(
            -scale(dpi, 12),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );
        let big_font = CreateFontW(
            -scale(dpi, 30),
            0,
            0,
            0,
            600,
            0,
            0,
            0,
            1,
            0,
            0,
            4,
            0,
            face.as_ptr(),
        );

        let pad = scale(dpi, 16);
        let right_block = scale(dpi, 124);

        let old = SelectObject(hdc, title_font);
        SetTextColor(hdc, CLR_TEXT);
        let mut title_rect = RECT {
            left: pad,
            top: scale(dpi, 12),
            right: client.right - right_block,
            bottom: scale(dpi, 36),
        };
        let title = wide(&state.plan.title);
        DrawTextW(
            hdc,
            title.as_ptr(),
            -1,
            &mut title_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(hdc, old);

        let old = SelectObject(hdc, body_font);
        SetTextColor(hdc, CLR_MUTED);
        let detail_text = if state.plan.detail.is_empty() {
            String::from("倒计时内按 Esc 可取消")
        } else {
            state.plan.detail.clone()
        };
        let mut detail_rect = RECT {
            left: pad,
            top: scale(dpi, 38),
            right: client.right - right_block,
            bottom: scale(dpi, 74),
        };
        let detail = wide(&detail_text);
        DrawTextW(
            hdc,
            detail.as_ptr(),
            -1,
            &mut detail_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );

        let status_text = match state.phase {
            0 => {
                let left = state.countdown_until.saturating_duration_since(Instant::now());
                format!("{}", (left.as_millis() + 999) / 1000)
            }
            1 => {
                let total = state.plan.steps.len();
                format!("{}/{}", (state.step + 1).min(total), total)
            }
            2 => String::from("完成"),
            _ => String::from("已取消"),
        };
        SelectObject(hdc, old);

        let old = SelectObject(hdc, big_font);
        SetTextColor(hdc, if state.phase == 3 { CLR_STOP } else { CLR_ACCENT });
        let mut status_rect = RECT {
            left: client.right - right_block,
            top: scale(dpi, 22),
            right: client.right - pad,
            bottom: scale(dpi, 62),
        };
        let status = wide(&status_text);
        DrawTextW(
            hdc,
            status.as_ptr(),
            -1,
            &mut status_rect,
            DT_RIGHT | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(hdc, old);

        let old = SelectObject(hdc, body_font);
        SetTextColor(hdc, CLR_MUTED);
        let line = wide("按 Esc 或点此取消");
        let mut line_rect = RECT {
            left: pad,
            top: client.bottom - scale(dpi, 30),
            right: client.right - right_block,
            bottom: client.bottom - scale(dpi, 8),
        };
        DrawTextW(
            hdc,
            line.as_ptr(),
            -1,
            &mut line_rect,
            DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(hdc, old);

        DeleteObject(title_font);
        DeleteObject(body_font);
        DeleteObject(big_font);
    }
}

/// GDI+ 画一个抗锯齿圆环。颜色用 ARGB。
fn gp_ring(g: isize, cx: f32, cy: f32, radius: f32, width: f32, argb: u32) {
    let mut pen: isize = 0;
    if unsafe { GdipCreatePen1(argb, width, 2, &mut pen) } != 0 {
        return;
    }
    let d = radius * 2.0;
    unsafe {
        GdipDrawEllipse(g, pen, cx - radius, cy - radius, d, d);
        GdipDeletePen(pen);
    }
}

fn gp_disc(g: isize, cx: f32, cy: f32, radius: f32, argb: u32) {
    let mut brush: isize = 0;
    if unsafe { GdipCreateSolidFill(argb, &mut brush) } != 0 {
        return;
    }
    let d = radius * 2.0;
    unsafe {
        GdipFillEllipse(g, brush, cx - radius, cy - radius, d, d);
        GdipDeleteBrush(brush);
    }
}

fn gp_line(g: isize, x1: f32, y1: f32, x2: f32, y2: f32, width: f32, argb: u32) {
    let mut pen: isize = 0;
    if unsafe { GdipCreatePen1(argb, width, 2, &mut pen) } != 0 {
        return;
    }
    unsafe {
        GdipDrawLine(g, pen, x1, y1, x2, y2);
        GdipDeletePen(pen);
    }
}

// 落点标记：圆里一个加号。圆用半透明灰填充，轮廓与加号都走双色描边
// （外深内浅），这样在深色与浅色背景上都能分开，不用逐像素反色。
fn draw_cue(hwnd: HWND, hdc: HDC) {
    if st().surface.is_some() {
        return;
    }
    let state = st();
    let radius = state.cue_radius;
    if radius < 1.0 {
        return;
    }
    unsafe {
        let mut client = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetClientRect(hwnd, &mut client);

        let clear = CreateSolidBrush(0x0000_0000);
        FillRect(hdc, &client, clear);
        DeleteObject(clear);

        let mut g: isize = 0;
        if GdipCreateFromHDC(hdc, &mut g) != 0 {
            return;
        }
        GdipSetSmoothingMode(g, 4);

        let cx = (client.right as f32) / 2.0;
        let cy = (client.bottom as f32) / 2.0;
        let u = state.dpi;

        let fill = if state.cue_pressed { A_CROSS } else { A_DISC };
        gp_disc(g, cx, cy, radius, fill);

        let arm = (radius * 0.46).max(scale(state.dpi, 6) as f32);
        let cross = if state.cue_pressed { A_DISC } else { A_CROSS };
        gp_line(g, cx - arm, cy, cx + arm, cy, 1.0, cross);
        gp_line(g, cx, cy - arm, cx, cy + arm, 1.0, cross);


        GdipDeleteGraphics(g);
    }
}
unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    match msg {
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut paint);
            // 双缓冲：先画到内存位图再整块贴回，读秒重绘才不会闪
            let mut client = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(hwnd, &mut client);
            let w = client.right.max(1);
            let h = client.bottom.max(1);
            let mem = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, w, h);
            if mem != 0 && bmp != 0 {
                let old = SelectObject(mem, bmp);
                draw_main(hwnd, mem);
                BitBlt(hdc, 0, 0, w, h, mem, 0, 0, 0x00CC_0020);
                SelectObject(mem, old);
                DeleteObject(bmp);
                DeleteDC(mem);
            } else {
                draw_main(hwnd, hdc);
            }
            EndPaint(hwnd, &paint);
            0
        }
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN => {
            let x = (lp & 0xFFFF) as i32;
            let y = ((lp >> 16) & 0xFFFF) as i32;
            let state = st();
            if x >= state.cancel_rect.left
                && x <= state.cancel_rect.right
                && y >= state.cancel_rect.top
                && y <= state.cancel_rect.bottom
            {
                cancel();
            }
            0
        }
        WM_TIMER => {
            tick();
            0
        }
        WM_DESTROY => {
            PostQuitMessage(st().exit_code);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn cue_proc(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    match msg {
        // 全屏覆盖层必须让鼠标穿透：WS_EX_TRANSPARENT 对非 layered 窗口不可靠，
        // 命中测试返回 HTTRANSPARENT 才能把点击与滚轮交还给下面的窗口。
        WM_NCHITTEST => -1,
        WM_PAINT => {
            // 挂了合成视觉树时内容由合成器负责，不走 BeginPaint，否则合成内容会被清掉
            if st().surface.is_some() {
                return 0;
            }
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut paint);
            draw_cue(hwnd, hdc);
            EndPaint(hwnd, &paint);
            0
        }
        WM_ERASEBKGND => 1,
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn hide_cue(hwnd: HWND) {
    if hwnd != 0 {
        unsafe {
            ShowWindow(hwnd, 0); // SW_HIDE
        }
    }
}

fn cancel() {
    let state = st();
    if state.phase == 3 || state.phase == 2 {
        return;
    }
    state.phase = 3;
    state.exit_code = 2;
    // 全屏覆盖层一结束就撤掉，避免它继续挡鼠标
    hide_cue(state.cue);
    state.done_until = Instant::now() + Duration::from_millis(700);
    invalidate(state.main);
}

/// 每帧把几何推给合成视觉树，坐标从物理像素换成 DIP。
fn push_surface(state: &mut State) {
    let surface = match &state.surface {
        Some(value) => value,
        None => return,
    };
    let scale = state.dpi;
    let ratio = (state.cue_radius / state.cue_from.max(1.0)).clamp(0.0, 1.0);
    let key = (
        state.cue_at.0 as f32 / scale,
        state.cue_at.1 as f32 / scale,
        state.cue_radius / scale,
        (1.0 + 2.0 * ratio) / scale,
        state.cue_pressed,
    );
    // 值没变就不重建，避免每帧 Clear 把末态刷掉
    if let Some(last) = state.last_push {
        if (last.0 - key.0).abs() < 0.05
            && (last.1 - key.1).abs() < 0.05
            && (last.2 - key.2).abs() < 0.05
            && (last.3 - key.3).abs() < 0.02
            && last.4 == key.4
        {
            return;
        }
    }
    state.last_push = Some(key);
    surface.update(
        state.cue_at.0 as f32 / scale,
        state.cue_at.1 as f32 / scale,
        state.cue_radius / scale,
        ((1.0 + 2.0 * ratio) / scale).max(1.4),
        state.cue_pressed,
    );
}

fn tick() {
    let state = st();
    let now = Instant::now();

    if unsafe { GetAsyncKeyState(VK_ESCAPE) } < 0 {
        cancel();
    }
    if let Some(at) = state.cancel_at {
        if now >= at {
            state.cancel_at = None;
            cancel();
        }
    }
    step_cue_animation();
    push_surface(st());

    match state.phase {
        0 => {
            if now >= state.countdown_until {
                eprintln!("countdown done after {:?}", now.duration_since(state.started));
                state.phase = 1;
                state.step = 0;
                state.wait_until = None;
                state.last_label.clear();
                invalidate(state.main);
                run_step();
            } else {
                let left = state.countdown_until.saturating_duration_since(now);
                let label = format!("{}", (left.as_millis() + 999) / 1000);
                if label != state.last_label {
                    state.last_label = label;
                    invalidate(state.main);
                }
            }
        }
        1 => {
            if let Some(deadline) = state.wait_until {
                if now < deadline {
                    return;
                }
                state.wait_until = None;
            }
            run_step();
        }
        2 | 3 => {
            if now >= state.done_until {
                unsafe {
                    DestroyWindow(state.main);
                }
            }
        }
        _ => {}
    }
}

fn run_step() {
    let state = st();
    if state.step >= state.plan.steps.len() {
        state.phase = 2;
        state.exit_code = 0;
        state.done_until = Instant::now() + Duration::from_millis(650);
        invalidate(state.main);
        return;
    }
    let step = state.plan.steps[state.step].clone();
    match step {
        Step::Move { x, y, .. } => {
            let moved = move_cursor(x, y);
            if !moved {
                // 本进程没有输入权限时，移动交给 exec 里的脚本完成；
                // 这里仍然把圆环画在目标坐标，用户能看到落点。
                eprintln!("move delegated: 本进程无输入权限，圆环标记 ({},{})", x, y);
            }
            place_cue(x, y, true);
            state.step += 1;
            // 让圆环在目标位置停留，既给视觉反馈，也给事件层时间刷新命中目标
            state.wait_until = Some(Instant::now() + Duration::from_millis(260));
        }
        Step::Click { double } => {
            let (x, y) = state.cue_at;
            state.cue_pressed = true;
            invalidate(state.cue);
            do_click(x, y, double);
            state.cue_pressed = false;
            invalidate(state.cue);
            state.step += 1;
            state.wait_until = Some(Instant::now() + Duration::from_millis(220));
        }
        Step::Exec { line } => {
            // 动作交给有输入权限的 powershell 执行，本进程只负责编排与提示
            let status = std::process::Command::new("cmd")
                .args(["/c", line.as_str()])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::inherit())
                .status();
            match status {
                Ok(code) if code.success() => eprintln!("exec ok: {}", line),
                Ok(code) => eprintln!("exec exit {} : {}", code.code().unwrap_or(-1), line),
                Err(err) => eprintln!("exec failed: {} ({})", err, line),
            }
            state.step += 1;
            state.wait_until = Some(Instant::now() + Duration::from_millis(220));
        }
        Step::Wait { ms } => {
            state.step += 1;
            state.wait_until = Some(Instant::now() + Duration::from_millis(ms));
        }
    }
    invalidate(state.main);
}

fn create_window(
    class_name: &str,
    title: &str,
    ex_style: u32,
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    proc_addr: usize,
    rounded: bool,
) -> HWND {
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class = wide(class_name);
        let wc = WNDCLASSW {
            style: 0,
            wnd_proc: proc_addr,
            cls_extra: 0,
            wnd_extra: 0,
            instance,
            icon: 0,
            cursor: LoadCursorW(0, 32512 as *const u16),
            background: 0,
            menu_name: std::ptr::null(),
            class_name: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let window_name = wide(title);
        let hwnd = CreateWindowExW(
            ex_style,
            class.as_ptr(),
            window_name.as_ptr(),
            WS_POPUP,
            x,
            y,
            w,
            h,
            0,
            0,
            instance,
            std::ptr::null_mut(),
        );
        // 黑色作色键透明，四角才不会被矩形窗口吃掉
        if rounded {
            // 圆角靠窗口区域裁剪，背景用实色填充，避免靠色键把面板也挖空
            SetLayeredWindowAttributes(hwnd, 0x0000_0000, 236, 2);
            let radius = 20;
            let rgn = CreateRoundRectRgn(0, 0, w + 1, h + 1, radius, radius);
            SetWindowRgn(hwnd, rgn, 1);
        } else {
            SetLayeredWindowAttributes(hwnd, 0x0000_0000, 0, 1);
        }
        hwnd
    }
}

fn run(plan: Plan, cancel_after: Option<u64>) -> i32 {
    let mut gp_token: usize = 0;
    let gp_input = GdiplusStartupInput {
        version: 1,
        debug_callback: 0,
        suppress_background: 0,
        suppress_codecs: 0,
    };
    let gp_ready = unsafe { GdiplusStartup(&mut gp_token, &gp_input, std::ptr::null_mut()) } == 0;
    eprintln!("gdiplus ready={}", gp_ready);
    // 窗口尚未创建时先探一次输入权限，便于区分「API 被拦」与「窗口抢了鼠标」
    let probe_ok = unsafe { SetCursorPos(200, 200) };
    let probe_err = unsafe { GetLastError() };
    std::thread::sleep(Duration::from_millis(30));
    eprintln!(
        "probe before window: setcursorpos={} err={} at={:?}",
        probe_ok,
        probe_err,
        cursor_at()
    );
    unsafe {
        let dpi = (GetDpiForSystem() as f32) / 96.0;
        let (sw, sh) = screen_size();
        let w = scale(dpi, 560);
        let h = scale(dpi, 96);
        let margin = scale(dpi, 16);
        let marker = scale(dpi, 120);
        let target_pt = plan.steps.iter().find_map(|step| match step {
            Step::Move { x, y, .. } => Some((*x, *y)),
            _ => None,
        });
        // 默认落在右上角；与目标点重叠时依次换到其余候选位置。
        let candidates = [
            (sw - w - margin, margin),
            (margin, margin),
            (sw - w - margin, sh - h - margin),
            (margin, sh - h - margin),
            ((sw - w) / 2, margin),
            ((sw - w) / 2, sh - h - margin),
        ];
        let (x, y) = match target_pt {
            Some((tx, ty)) => {
                let mut chosen = candidates[0];
                for (cx, cy) in candidates {
                    let overlap = tx + marker > cx && tx - marker < cx + w && ty + marker > cy && ty - marker < cy + h;
                    if !overlap {
                        chosen = (cx, cy);
                        break;
                    }
                }
                chosen
            }
            None => candidates[0],
        };

        let proc_addr = main_proc as usize;
        let main = create_window(
            "MouseCueMain",
            "mouse-cue",
            WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w,
            h,
            x,
            y,
            proc_addr,
            true,
        );

        let cue_size = scale(dpi, 120);
        let cue = create_window(
            "MouseCueRing",
            "mouse-cue-ring",
            WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | 0x0020_0000,
            screen_size().0 - 2,
            screen_size().1 - 2,
            1,
            1,
            cue_proc as usize,
            false,
        );

        // 合成视觉树接管标记绘制
        let surface = match dcomp::MarkerSurface::attach(cue) {
            Some(value) => {
                eprintln!("dcomp: surface ready");
                Some(value)
            }
            None => {
                eprintln!("dcomp: attach failed, falling back to gdi");
                None
            }
        };

        let started = Instant::now();
        let countdown_until = started + Duration::from_secs(plan.countdown);
        eprintln!(
            "plan countdown={} steps={} window=({},{} {}x{}) dpi={}",
            plan.countdown,
            plan.steps.len(),
            x,
            y,
            w,
            h,
            dpi
        );
        let cancel_rect = RECT {
            left: w - scale(dpi, 156),
            top: scale(dpi, 8),
            right: w - scale(dpi, 8),
            bottom: scale(dpi, 30),
        };

        STATE = Some(State {
            plan,
            cancel_at: cancel_after.map(|secs| started + Duration::from_secs(secs)),
            phase: 0,
            countdown_until,
            step: 0,
            wait_until: None,
            done_until: started,
            started,
            main,
            cue,
            cue_at: (0, 0),
            cue_pressed: false,
            cue_from: 0.0,
            cue_to: 0.0,
            cue_started: Instant::now(),
            cue_ms: 520.0,
            cue_radius: 0.0,
            disc_color: 0xFF3C_3C3C,
            cross_color: 0xFFF4_F4F4,
            cancel_rect,
            dpi,
            exit_code: 0,
            last_label: String::new(),
            surface,
            last_push: None,
        });

        ShowWindow(main, SW_SHOWNOACTIVATE);
        UpdateWindow(main);
        // 倒计时一开始就把落点标出来，用户才有时间看清要动哪里
        if let Some((tx, ty)) = target_pt {
            place_cue(tx, ty, true);
            move_cursor(tx, ty);
        }
        SetWindowPos(
            main,
            HWND_TOPMOST,
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        SetTimer(main, 1, 16, 0);

        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let state = st();
        KillTimer(state.main, 1);
        let elapsed = Instant::now().duration_since(state.started).as_millis();
        let code = state.exit_code;
        let steps = state.plan.steps.len();
        if gp_ready {
            GdiplusShutdown(gp_token);
        }
        // 输出给调用方：一行 JSON
        if code == 2 {
            println!(
                "{{\"status\":\"cancelled\",\"reason\":\"user\",\"step\":{},\"steps\":{},\"elapsedMs\":{}}}",
                state.step, steps, elapsed
            );
            eprintln!("操作失败: 用户取消");
        } else if code == 0 {
            println!(
                "{{\"status\":\"ok\",\"step\":{},\"steps\":{},\"elapsedMs\":{}}}",
                steps, steps, elapsed
            );
        } else {
            println!(
                "{{\"status\":\"error\",\"code\":{},\"steps\":{},\"elapsedMs\":{}}}",
                code, steps, elapsed
            );
        }
        code
    }
}

fn main() {
    unsafe {
        SetProcessDPIAware();
    }
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: mouse-cue <plan.txt>  或  mouse-cue --demo");
        std::process::exit(1);
    }

    let plan = if args[1] == "--demo" {
        parse_plan(
            "title=示例\n\
             detail=显示提示后移动到屏幕中央并点击\n\
             countdown=3\n\
             move=960,540,屏幕中央\n\
             click\n\
             wait=600\n",
        )
        .unwrap()
    } else {
        match fs::read_to_string(&args[1]) {
            Ok(text) => match parse_plan(&text) {
                Ok(plan) => plan,
                Err(err) => {
                    eprintln!("计划文件有误: {}", err);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("读不到计划文件 {}: {}", args[1], err);
                std::process::exit(1);
            }
        }
    };

    let cancel_after = if args.iter().any(|a| a == "--selftest-cancel") { Some(2) } else { None };
    let code = run(plan, cancel_after);
    std::process::exit(code);
}
