// 工具配置：落在 exe 同目录的 mouse-cue.ini，键值形式，缺项回落到默认值。
// 这里只管读取与保存，界面在 settings.rs。

use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// 倒计时秒数
    pub countdown: u64,
    /// 标记收缩动画时长
    pub cue_ms: f32,
    /// 落点线段的收尾粗细（DIP）
    pub line_min: f32,
    /// 落点线段的起始粗细（DIP）
    pub line_max: f32,
    /// 落点收缩后的半长（DIP）
    pub cue_half: f32,
    /// 落点从屏幕边缘开始收缩
    pub cue_from_screen: bool,
    /// 像素颜色，0xRRGGBB
    pub line_color: u32,
    /// 浮层面板不透明度 0-255
    pub panel_alpha: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            countdown: 3,
            cue_ms: 520.0,
            line_min: 1.4,
            line_max: 3.0,
            cue_half: 15.0,
            cue_from_screen: true,
            line_color: 0x00F2_F2F2,
            panel_alpha: 236,
        }
    }
}

pub fn config_path() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("mouse-cue.exe"));
    exe.parent()
        .map(|dir| dir.join("mouse-cue.ini"))
        .unwrap_or_else(|| PathBuf::from("mouse-cue.ini"))
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();
        let text = match fs::read_to_string(config_path()) {
            Ok(value) => value,
            Err(_) => return config,
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let (key, value) = match line.split_once('=') {
                Some(pair) => (pair.0.trim(), pair.1.trim()),
                None => continue,
            };
            match key {
                "countdown" => {
                    if let Ok(parsed) = value.parse() {
                        config.countdown = parsed;
                    }
                }
                "cue_ms" => {
                    if let Ok(parsed) = value.parse() {
                        config.cue_ms = parsed;
                    }
                }
                "line_min" => {
                    if let Ok(parsed) = value.parse() {
                        config.line_min = parsed;
                    }
                }
                "line_max" => {
                    if let Ok(parsed) = value.parse() {
                        config.line_max = parsed;
                    }
                }
                "cue_half" => {
                    if let Ok(parsed) = value.parse() {
                        config.cue_half = parsed;
                    }
                }
                "cue_from_screen" => {
                    config.cue_from_screen = value == "1" || value.eq_ignore_ascii_case("true");
                }
                "line_color" => {
                    let digits = value.trim_start_matches("0x").trim_start_matches("0X");
                    if let Ok(parsed) = u32::from_str_radix(digits, 16) {
                        config.line_color = parsed & 0x00FF_FFFF;
                    }
                }
                "panel_alpha" => {
                    if let Ok(parsed) = value.parse() {
                        config.panel_alpha = parsed;
                    }
                }
                _ => {}
            }
        }
        config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let text = format!(
            "# mouse-cue 配置\ncountdown={}\ncue_ms={}\nline_min={}\nline_max={}\ncue_half={}\ncue_from_screen={}\nline_color=0x{:06X}\npanel_alpha={}\n",
            self.countdown,
            self.cue_ms,
            self.line_min,
            self.line_max,
            self.cue_half,
            if self.cue_from_screen { 1 } else { 0 },
            self.line_color,
            self.panel_alpha
        );
        fs::write(config_path(), text)
    }
}
