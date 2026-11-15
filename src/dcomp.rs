// 用 Windows.UI.Composition 在普通 HWND 上挂一棵视觉树，画两条交叉线。
// 走合成器不需要色键或 UpdateLayeredWindow：透明由合成处理，全屏尺寸也能正常出图。
//
// 线用细矩形拼，不用 CompositionLineGeometry：后者没有起止点接口，
// 而矩形几何与 SpriteShape 的组合已经验证可以稳定显示。

use windows::core::Interface;
use windows::System::DispatcherQueueController;
use windows::UI::Color;
use windows::UI::Composition::Desktop::DesktopWindowTarget;
use windows::UI::Composition::{
    Compositor, CompositionColorBrush, CompositionRectangleGeometry, CompositionSpriteShape, ContainerVisual,
    ShapeVisual,
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
    brush: CompositionColorBrush,
    h_geo: CompositionRectangleGeometry,
    v_geo: CompositionRectangleGeometry,
    h_sprite: CompositionSpriteShape,
    v_sprite: CompositionSpriteShape,
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

        let h_geo = compositor.CreateRectangleGeometry().ok()?;
        let v_geo = compositor.CreateRectangleGeometry().ok()?;
        let h_sprite = compositor.CreateSpriteShapeWithGeometry(&h_geo).ok()?;
        let v_sprite = compositor.CreateSpriteShapeWithGeometry(&v_geo).ok()?;
        h_sprite.SetFillBrush(&brush).ok()?;
        v_sprite.SetFillBrush(&brush).ok()?;

        let shapes = shape.Shapes().ok()?;
        shapes.Append(&h_sprite).ok()?;
        shapes.Append(&v_sprite).ok()?;

        Some(Self {
            _queue: queue,
            _target: target,
            compositor,
            root,
            shape,
            brush,
            h_geo,
            v_geo,
            h_sprite,
            v_sprite,
        })
    }

    /// 逐帧更新。中心点与半长都是 DIP，width 是线的粗细。
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
        let w = 20.0; // 临时放大，验证是否只是太细
        eprintln!(
            "dcomp update cx={} cy={} half={} w={} size=({},{})",
            cx, cy, half, w, half * 2.0, w
        );

        let _ = self.h_geo.SetSize(Vector2 {
            X: (half * 2.0).max(1.0),
            Y: w,
        });
        let _ = self.h_sprite.SetOffset(Vector2 {
            X: cx - half,
            Y: cy - w / 2.0,
        });

        let _ = self.v_geo.SetSize(Vector2 {
            X: w,
            Y: (half * 2.0).max(1.0),
        });
        let _ = self.v_sprite.SetOffset(Vector2 {
            X: cx - w / 2.0,
            Y: cy - half,
        });
    }

    pub fn set_visible(&self, visible: bool) {
        let _ = self.shape.SetIsVisible(visible);
    }
}
