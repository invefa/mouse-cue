// 用 Windows.UI.Composition 在普通 HWND 上挂一棵视觉树，画两条交叉线。
// 走合成器不需要色键或 UpdateLayeredWindow：透明由合成处理，全屏尺寸也能正常出图。
//
// 线用细矩形拼，不用 CompositionLineGeometry：后者没有起止点接口。
// 每帧重建 geometry 与 sprite：预建对象上改 SetSize/SetOffset 实测不重绘，
// 一次只画两条线，重建的开销可以接受。

use std::cell::RefCell;

use windows::core::Interface;
use windows::System::DispatcherQueueController;
use windows::UI::Color;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{
    Compositor, CompositionColorBrush, CompositionRectangleGeometry, CompositionShapeCollection,
    CompositionSpriteShape, ContainerVisual, ShapeVisual,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED};
use windows_numerics::Vector2;

#[repr(C)]
struct DispatcherQueueOptions {
    size: u32,
    thread_type: u32,
    apartment_type: u32,
}

#[link(name = "CoreMessaging")]
extern "system" {
    fn CreateDispatcherQueueController(
        options: DispatcherQueueOptions,
        controller: *mut *mut core::ffi::c_void,
    ) -> i32;
}

/// 在当前线程建 DispatcherQueue。windows crate 只提供「独立线程」的构造，
/// 而 Compositor 要求调用线程自己已经初始化过队列，这里直接调底层接口。
fn queue_on_current_thread() -> Option<DispatcherQueueController> {
    const DQTYPE_THREAD_CURRENT: u32 = 2;
    const DQTAT_COM_ASTA: u32 = 2;
    let options = DispatcherQueueOptions {
        size: core::mem::size_of::<DispatcherQueueOptions>() as u32,
        thread_type: DQTYPE_THREAD_CURRENT,
        apartment_type: DQTAT_COM_ASTA,
    };
    let mut raw: *mut core::ffi::c_void = core::ptr::null_mut();
    let status = unsafe { CreateDispatcherQueueController(options, &mut raw) };
    if status < 0 || raw.is_null() {
        eprintln!("dcomp: CreateDispatcherQueueController failed 0x{:08x}", status);
        return None;
    }
    Some(unsafe { DispatcherQueueController::from_raw(raw as *mut core::ffi::c_void) })
}

pub struct MarkerSurface {
    // 下面两个必须持有：队列一销毁合成器失效，目标一释放整棵树关闭
    _queue: DispatcherQueueController,
    _target: DesktopWindowTarget,
    compositor: Compositor,
    root: ContainerVisual,
    shape: ShapeVisual,
    shapes: CompositionShapeCollection,
    brush: CompositionColorBrush,
    // 必须持有当前这帧与上一帧用到的 geometry 与 sprite：
    // 它们是 COM 对象，一旦引用归零就被关闭，合成器随即丢弃画面。
    alive: RefCell<Vec<(CompositionRectangleGeometry, CompositionSpriteShape)>>,
}

impl MarkerSurface {
    /// 在 hwnd 上建立视觉树。窗口需要是不带 layered 的普通窗口。
    pub fn attach(hwnd: isize) -> Option<Self> {
        let hwnd = HWND(hwnd as *mut core::ffi::c_void);
        unsafe {
            let _ = RoInitialize(RO_INIT_SINGLETHREADED);
        }
        let queue = queue_on_current_thread()?;
        let compositor = match Compositor::new() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("dcomp: compositor failed: {}", error);
                return None;
            }
        };
        let interop: ICompositorDesktopInterop = match compositor.cast() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("dcomp: interop cast failed: {}", error);
                return None;
            }
        };
        let target = match unsafe { interop.CreateDesktopWindowTarget(hwnd, false) } {
            Ok(value) => value,
            Err(error) => {
                eprintln!("dcomp: desktop target failed: {}", error);
                return None;
            }
        };

        let root = compositor.CreateContainerVisual().ok()?;
        target.SetRoot(&root).ok()?;

        let (sw, sh) = crate::screen_size();
        let scale = crate::dpi_scale();
        let shape = compositor.CreateShapeVisual().ok()?;
        shape
            .SetSize(Vector2 {
                X: sw as f32 / scale,
                Y: sh as f32 / scale,
            })
            .ok()?;
        root.Children().ok()?.InsertAtTop(&shape).ok()?;

        let brush = compositor
            .CreateColorBrushWithColor(Color {
                A: 255,
                R: 242,
                G: 242,
                B: 242,
            })
            .ok()?;
        let shapes = shape.Shapes().ok()?;

        Some(Self {
            _queue: queue,
            _target: target,
            compositor,
            root,
            shape,
            shapes,
            brush,
            alive: RefCell::new(Vec::new()),
        })
    }

    /// 逐帧重建两条线。中心点、半长与粗细都是 DIP 单位。
    pub fn update(&self, cx: f32, cy: f32, half: f32, width: f32, pressed: bool) {
        if pressed {
            let _ = self.brush.SetColor(Color {
                A: 255,
                R: 255,
                G: 255,
                B: 255,
            });
        }
        let w = width.clamp(0.8, 40.0);
        let _ = self.shapes.Clear();
        self.alive.borrow_mut().clear();

        self.push_bar(cx - half, cy - w / 2.0, half * 2.0, w);
        self.push_bar(cx - w / 2.0, cy - half, w, half * 2.0);
    }

    fn push_bar(&self, x: f32, y: f32, w: f32, h: f32) {
        let geo = match self.compositor.CreateRectangleGeometry() {
            Ok(value) => value,
            Err(_) => return,
        };
        let _ = geo.SetSize(Vector2 { X: w.max(1.0), Y: h.max(1.0) });
        let sprite: CompositionSpriteShape = match self.compositor.CreateSpriteShapeWithGeometry(&geo) {
            Ok(value) => value,
            Err(_) => return,
        };
        let _ = sprite.SetOffset(Vector2 { X: x, Y: y });
        let _ = sprite.SetFillBrush(&self.brush);
        let _ = self.shapes.Append(&sprite);
        self.alive.borrow_mut().push((geo, sprite));
    }

    pub fn debug_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        self.push_bar(x, y, w, h);
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.shape.SetIsVisible(visible);
    }
}
