// 用 Windows.UI.Composition 在普通 HWND 上挂一棵视觉树，画两条横跨窗口的交叉线。
// 走这条路不需要 SetLayeredWindowAttributes，也不需要 UpdateLayeredWindow：
// 透明由合成器处理，线的粗细与长度可以逐帧改，全屏尺寸也不会像色键窗口那样画不出来。

use windows::core::Interface;
use windows_numerics::Vector2;
use windows::UI::Color;
use windows::Win32::System::WinRT::Composition::ICompositorDesktopInterop;
use windows::UI::Composition::{
    Compositor, CompositionColorBrush, CompositionLineGeometry, CompositionSpriteShape, ContainerVisual,
    ShapeVisual,
};
use windows::System::DispatcherQueueController;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED};


pub struct MarkerSurface {
    // 必须持有：DispatcherQueue 一销毁，合成器随之失效
    _queue: DispatcherQueueController,
    compositor: Compositor,
    root: ContainerVisual,
    shape: ShapeVisual,
    brush: CompositionColorBrush,
    horizontal: CompositionSpriteShape,
    vertical: CompositionSpriteShape,
    h_line: CompositionLineGeometry,
    v_line: CompositionLineGeometry,
}

impl MarkerSurface {
    /// 在 hwnd 上建立视觉树。失败时返回 None，调用方回退到旧画法。
    pub fn attach(hwnd: isize) -> Option<Self> {
        let hwnd = HWND(hwnd as *mut core::ffi::c_void);
        // WinRT 合成器要求当前线程有 COM 与 DispatcherQueue
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
        root.SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 }).ok()?;
        target.SetRoot(&root).ok()?;

        let shape = compositor.CreateShapeVisual().ok()?;
        shape.SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 }).ok()?;
        root.Children().ok()?.InsertAtTop(&shape).ok()?;

        let brush = compositor
            .CreateColorBrushWithColor(Color {
                A: 255,
                R: 242,
                G: 242,
                B: 242,
            })
            .ok()?;

        let h_line = compositor.CreateLineGeometry().ok()?;
        let v_line = compositor.CreateLineGeometry().ok()?;

        let horizontal = compositor.CreateSpriteShapeWithGeometry(&h_line).ok()?;
        horizontal.SetStrokeBrush(&brush).ok()?;
        horizontal.SetFillBrush(&brush).ok()?;

        let vertical = compositor.CreateSpriteShapeWithGeometry(&v_line).ok()?;
        vertical.SetStrokeBrush(&brush).ok()?;
        vertical.SetFillBrush(&brush).ok()?;

        let shapes = shape.Shapes().ok()?;
        shapes.Append(&horizontal).ok()?;
        shapes.Append(&vertical).ok()?;

        Some(Self {
            _queue: queue,
            compositor,
            root,
            shape,
            brush,
            horizontal,
            vertical,
            h_line,
            v_line,
        })
    }

    /// 逐帧更新：中心点用 DIP 坐标，half 是当前半长。
    pub fn update(&self, cx: f32, cy: f32, half: f32, width: f32, pressed: bool) {
        let color = if pressed {
            Color {
                A: 255,
                R: 255,
                G: 255,
                B: 255,
            }
        } else {
            Color {
                A: 255,
                R: 242,
                G: 242,
                B: 242,
            }
        };
        let _ = self.brush.SetColor(color);
        let _ = self
            .h_line
            .SetStart(Vector2 { X: cx - half, Y: cy });
        let _ = self
            .h_line
            .SetEnd(Vector2 { X: cx + half, Y: cy });
        let _ = self
            .v_line
            .SetStart(Vector2 { X: cx, Y: cy - half });
        let _ = self
            .v_line
            .SetEnd(Vector2 { X: cx, Y: cy + half });
        let _ = self.horizontal.SetStrokeThickness(width);
        let _ = self.vertical.SetStrokeThickness(width);
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.shape.SetIsVisible(visible);
    }

    pub fn resize(&self, width: f32, height: f32) {
        let _ = self.shape.SetSize(Vector2 {
            X: width,
            Y: height,
        });
    }
}

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