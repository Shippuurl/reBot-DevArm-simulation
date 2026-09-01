//! Shared visual tokens for the desktop workspace.
//!
//! The values mirror the useful parts of shadcn's token approach while staying
//! native to the egui version used by this application. Keeping the palette in
//! one module makes future panels consistent without coupling them to a widget
//! crate.

use eframe::egui::{Color32, Context, FontData, FontDefinitions, FontFamily};

// Neutral dark surfaces and semantic colors follow shadcn's dark palette.
pub const BG: Color32 = Color32::from_rgb(11, 14, 17);
pub const PANEL: Color32 = Color32::from_rgb(16, 21, 25);
pub const PANEL_ALT: Color32 = Color32::from_rgb(23, 30, 36);
pub const BORDER: Color32 = Color32::from_rgb(39, 47, 55);
pub const TEXT: Color32 = Color32::from_rgb(242, 244, 247);
pub const MUTED: Color32 = Color32::from_rgb(152, 163, 174);
pub const ACCENT: Color32 = Color32::from_rgb(104, 160, 255);
pub const OK: Color32 = Color32::from_rgb(86, 211, 157);
#[allow(dead_code)]
pub const WARN: Color32 = Color32::from_rgb(244, 184, 106);
pub const DANGER: Color32 = Color32::from_rgb(240, 113, 120);

#[allow(dead_code)]
pub const SPACE_XS: f32 = 4.0;
#[allow(dead_code)]
pub const SPACE_SM: f32 = 6.0;
#[allow(dead_code)]
pub const SPACE_MD: f32 = 8.0;
#[allow(dead_code)]
pub const SPACE_LG: f32 = 12.0;
#[allow(dead_code)]
pub const SPACE_XL: f32 = 16.0;
pub const PANEL_INSET: f32 = 12.0;
pub const CONTROL_HEIGHT: f32 = 32.0;
pub const NAV_ROW_HEIGHT: f32 = 30.0;

/// Install the bundled UI and telemetry fonts.
///
/// Inter is used for controls and labels; JetBrains Mono is used for values,
/// timestamps and other data that should remain column-aligned.
pub fn configure_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Inter-Regular".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")).into(),
    );
    fonts.font_data.insert(
        "Inter-Bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/Inter-Bold.ttf")).into(),
    );
    fonts.font_data.insert(
        "JetBrainsMono-Regular".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "JetBrainsMono-Medium".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf")).into(),
    );
    fonts.font_data.insert(
        "NotoSansSC".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/NotoSansSC-VF.ttf")).into(),
    );

    if let Some(proportional) = fonts.families.get_mut(&FontFamily::Proportional) {
        proportional.clear();
        proportional.push("Inter-Regular".to_owned());
        proportional.push("Inter-SemiBold".to_owned());
        proportional.push("Inter-Bold".to_owned());
        proportional.push("NotoSansSC".to_owned());
    }
    if let Some(monospace) = fonts.families.get_mut(&FontFamily::Monospace) {
        monospace.clear();
        monospace.push("JetBrainsMono-Regular".to_owned());
        monospace.push("JetBrainsMono-Medium".to_owned());
        monospace.push("NotoSansSC".to_owned());
    }

    ctx.set_fonts(fonts);
}
