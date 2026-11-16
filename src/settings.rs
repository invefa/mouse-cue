// 设置窗口：Win32 窗口 + OpenGL 上下文 + imgui(glow)。
// imgui 负责界面，渲染交给 imgui-glow-renderer，输入自己从窗口消息映射。

use std::ffi::{c_void, CString};

use imgui::{Condition, Context, Ui};
use imgui_glow_renderer::glow;
use imgui_glow_renderer::AutoRenderer;
use windows::core::s;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, HDC};
use windows::Win32::Graphics::OpenGL::{
    wglCreateContext, wglGetProcAddress, wglMakeCurrent, ChoosePixelFormat, SetPixelFormat,
    SwapBuffers, HGLRC, PFD_DOUBLEBUFFER, PFD_DRAW_TO_WINDOW, PFD_SUPPORT_OPENGL, PFD_TYPE_RGBA,
    PIXELFORMATDESCRIPTOR,
};
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, PeekMessageW,
    RegisterClassW, TranslateMessage, CS_HREDRAW, CS_VREDRAW, MSG, PM_REMOVE, WM_CHAR, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_QUIT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WNDCLASSW, WS_CAPTION, WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE,
};

use crate::config::Config;

static mut MOUSE: (f32, f32) = (0.0, 0.0);
static mut MOUSE_DOWN: bool = false;
static mut RIGHT_DOWN: bool = false;
static mut WHEEL: f32 = 0.0;
static mut CHARS: Vec<char> = Vec::new();
static mut KEYS: Vec<(imgui::Key, bool)> = Vec::new();

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// OpenGL 函数加载器：先问 wglGetProcAddress，取不到再查 opengl32.dll 的导出表。
fn load_gl(name: &str) -> *const c_void {
    let cname = match CString::new(name) {
        Ok(value) => value,
        Err(_) => return std::ptr::null(),
    };
    unsafe {
        if let Some(proc) = wglGetProcAddress(windows::core::PCSTR(cname.as_ptr() as *const u8)) {
            let raw = proc as isize;
            if raw != 1 && raw != 2 && raw != 3 && raw != -1 {
                return proc as *const c_void;
            }
        }
        let module = GetModuleHandleA(s!("opengl32.dll")).unwrap_or_default();
        match GetProcAddress(module, windows::core::PCSTR(cname.as_ptr() as *const u8)) {
            Some(value) => value as *const c_void,
            None => std::ptr::null(),
        }
    }
}

fn create_gl_context(hwnd: HWND) -> Option<(HDC, HGLRC)> {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        if hdc.is_invalid() {
            eprintln!("settings: GetDC failed");
            return None;
        }
        let format = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cAlphaBits: 8,
            cDepthBits: 24,
            ..Default::default()
        };
        let index = ChoosePixelFormat(hdc, &format);
        if index == 0 {
            eprintln!("settings: ChoosePixelFormat failed");
            return None;
        }
        if SetPixelFormat(hdc, index, &format).is_err() {
            eprintln!("settings: SetPixelFormat failed");
            return None;
        }
        let rc = match wglCreateContext(hdc) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("settings: wglCreateContext failed: {}", error);
                return None;
            }
        };
        if wglMakeCurrent(hdc, rc).is_err() {
            eprintln!("settings: wglMakeCurrent failed");
            return None;
        }
        Some((hdc, rc))
    }
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_MOUSEMOVE => {
            MOUSE = (
                (lp.0 & 0xFFFF) as i16 as f32,
                ((lp.0 >> 16) & 0xFFFF) as i16 as f32,
            );
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            MOUSE_DOWN = true;
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            MOUSE_DOWN = false;
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            RIGHT_DOWN = true;
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            RIGHT_DOWN = false;
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            WHEEL += ((wp.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
            LRESULT(0)
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(wp.0 as u32) {
                CHARS.push(ch);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if let Some(key) = map_key(wp.0 as u16) {
                KEYS.push((key, true));
            }
            LRESULT(0)
        }
        WM_KEYUP => {
            if let Some(key) = map_key(wp.0 as u16) {
                KEYS.push((key, false));
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn map_key(key: u16) -> Option<imgui::Key> {
    match key {
        0x0D => Some(imgui::Key::Enter),
        0x1B => Some(imgui::Key::Escape),
        0x20 => Some(imgui::Key::Space),
        0x09 => Some(imgui::Key::Tab),
        0x08 => Some(imgui::Key::Backspace),
        0x25 => Some(imgui::Key::LeftArrow),
        0x26 => Some(imgui::Key::UpArrow),
        0x27 => Some(imgui::Key::RightArrow),
        0x28 => Some(imgui::Key::DownArrow),
        _ => None,
    }
}

/// 打开设置窗口，返回是否点了保存。
pub fn run(config: &mut Config) -> bool {
    unsafe {
        let class_name = wide("MouseCueSettings");
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_proc),
            hInstance: instance.into(),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        let title = wide("mouse-cue 设置");
        let hwnd = match CreateWindowExW(
            Default::default(),
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            120,
            120,
            580,
            560,
            None,
            None,
            Some(instance.into()),
            None,
        ) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("settings: CreateWindowEx failed: {}", error);
                return false;
            }
        };

        // 显式显示：只靠 WS_VISIBLE 有时窗口不出现
        let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(hwnd, windows::Win32::UI::WindowsAndMessaging::SW_SHOW);
        let (hdc, glrc) = match create_gl_context(hwnd) {
            Some(pair) => pair,
            None => {
                let _ = DestroyWindow(hwnd);
                return false;
            }
        };

        let gl = glow::Context::from_loader_function(load_gl);
        let mut context = Context::create();
        {
            let style = context.style_mut();
            style.anti_aliased_lines = true;
            style.anti_aliased_fill = true;
            // 中性灰：层级靠线宽与间距，滑块与按钮不用重色填充
            style.window_rounding = 0.0;
            style.frame_rounding = 2.0;
            style.grab_rounding = 2.0;
            style.scrollbar_rounding = 2.0;
            style.colors[imgui::StyleColor::WindowBg as usize] = [0.07, 0.07, 0.07, 1.0];
            style.colors[imgui::StyleColor::ChildBg as usize] = [0.10, 0.10, 0.10, 1.0];
            style.colors[imgui::StyleColor::FrameBg as usize] = [0.15, 0.15, 0.15, 1.0];
            style.colors[imgui::StyleColor::FrameBgHovered as usize] = [0.21, 0.21, 0.21, 1.0];
            style.colors[imgui::StyleColor::FrameBgActive as usize] = [0.26, 0.26, 0.26, 1.0];
            style.colors[imgui::StyleColor::Button as usize] = [0.15, 0.15, 0.15, 1.0];
            style.colors[imgui::StyleColor::ButtonHovered as usize] = [0.23, 0.23, 0.23, 1.0];
            style.colors[imgui::StyleColor::ButtonActive as usize] = [0.30, 0.30, 0.30, 1.0];
            style.colors[imgui::StyleColor::Text as usize] = [0.87, 0.87, 0.87, 1.0];
            style.colors[imgui::StyleColor::TextDisabled as usize] = [0.48, 0.48, 0.48, 1.0];
            style.colors[imgui::StyleColor::SliderGrab as usize] = [0.62, 0.62, 0.62, 1.0];
            style.colors[imgui::StyleColor::SliderGrabActive as usize] = [0.92, 0.92, 0.92, 1.0];
            style.colors[imgui::StyleColor::CheckMark as usize] = [0.92, 0.92, 0.92, 1.0];
            style.frame_border_size = 0.0;
            style.window_border_size = 0.0;
            style.item_spacing = [10.0, 7.0];
            style.frame_padding = [6.0, 3.0];
            style.window_padding = [16.0, 14.0];
        }

        // 默认字体只有 ASCII，中文会显示成问号；挂一个系统中文字体并限制到常用汉字
        if let Ok(data) = std::fs::read("C:/Windows/Fonts/Deng.ttf") {
            let ranges = imgui::FontGlyphRanges::chinese_simplified_common();
            let config = imgui::FontConfig {
                size_pixels: 17.0,
                glyph_ranges: ranges,
                oversample_h: 2,
                oversample_v: 2,
                ..Default::default()
            };
            context.fonts().add_font(&[imgui::FontSource::TtfData {
                data: &data,
                size_pixels: 17.0,
                config: Some(config),
            }]);
        }

        let mut renderer = match AutoRenderer::initialize(gl, &mut context) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("settings: renderer init failed: {:?}", error);
                let _ = DestroyWindow(hwnd);
                return false;
            }
        };

        let mut saved = false;
        let mut running = true;
        let mut last = std::time::Instant::now();

        while running {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    running = false;
                    break;
                }
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
            if !running {
                break;
            }

            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            let width = (client.right - client.left).max(1) as f32;
            let height = (client.bottom - client.top).max(1) as f32;

            let now = std::time::Instant::now();
            let delta = now.duration_since(last).as_secs_f32();
            last = now;

            let io = context.io_mut();
            io.display_size = [width, height];
            io.delta_time = delta.max(0.001);
            io.mouse_pos = [MOUSE.0, MOUSE.1];
            io.mouse_down[0] = MOUSE_DOWN;
            io.mouse_down[1] = RIGHT_DOWN;
            io.mouse_wheel = WHEEL;
            WHEEL = 0.0;
            for ch in CHARS.drain(..) {
                io.add_input_character(ch);
            }
            for (key, down) in KEYS.drain(..) {
                io.add_key_event(key, down);
            }

            let ui = context.frame();
            imgui::Window::new(&ui, "mouse-cue")
                .size([width, height], Condition::Always)
                .position([0.0, 0.0], Condition::Always)
                .title_bar(false)
                .resizable(false)
                .movable(false)
                .build(|| {
                    draw_ui(&ui, config, &mut saved, &mut running);
                });

            let draw_data = context.render();
            if let Err(error) = renderer.render(draw_data) {
                eprintln!("settings: render failed: {:?}", error);
                break;
            }
            let _ = SwapBuffers(hdc);
            std::thread::sleep(std::time::Duration::from_millis(8));
        }

        let _ = wglMakeCurrent(hdc, HGLRC::default());
        let _ = DestroyWindow(hwnd);
        saved
    }
}

fn draw_ui(ui: &Ui, config: &mut Config, saved: &mut bool, running: &mut bool) {
    ui.spacing();
    ui.text("行为");
    ui.separator();

    let mut countdown = config.countdown as i32;
    if ui.slider("倒计时（秒）", 1, 10, &mut countdown) {
        config.countdown = countdown.clamp(1, 10) as u64;
    }

    let mut cue_ms = config.cue_ms;
    if ui.slider("收缩时长（毫秒）", 120.0, 1200.0, &mut cue_ms) {
        config.cue_ms = cue_ms;
    }

    ui.spacing();
    ui.text("标记");
    ui.separator();

    let mut line_max = config.line_max;
    if ui.slider("起始线宽", 1.0, 8.0, &mut line_max) {
        config.line_max = line_max;
    }

    let mut line_min = config.line_min;
    if ui.slider("收尾线宽", 0.8, 4.0, &mut line_min) {
        config.line_min = line_min;
    }

    let mut half = config.cue_half;
    if ui.slider("收尾半长", 8.0, 40.0, &mut half) {
        config.cue_half = half;
    }

    let mut from_screen = config.cue_from_screen;
    if ui.checkbox("从屏幕边缘开始收缩", &mut from_screen) {
        config.cue_from_screen = from_screen;
    }

    ui.spacing();
    ui.text("浮层");
    ui.separator();

    let mut alpha = config.panel_alpha as i32;
    if ui.slider("不透明度", 120, 255, &mut alpha) {
        config.panel_alpha = alpha as u8;
    }

    ui.spacing();
    ui.spacing();

    if ui.button("保存") {
        match config.save() {
            Ok(_) => *saved = true,
            Err(error) => eprintln!("settings: save failed: {}", error),
        }
        *running = false;
    }
    ui.same_line();
    if ui.button("取消") {
        *running = false;
    }
}
