use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use qrcode::QrCode;
use qrcode::render::{svg, unicode};

/// Renders the QR as Unicode half-blocks for the terminal.
///
/// Colors are inverted so the code stays readable on a dark terminal.
pub fn render_terminal(text: &str) -> Result<String, String> {
    let code = QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build())
}

/// Renders the QR as an SVG `data:` URL for the desktop GUI.
pub fn render_svg_data_url(text: &str) -> Result<String, String> {
    let code = QrCode::new(text.as_bytes()).map_err(|e| e.to_string())?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .build();
    Ok(format!("data:image/svg+xml;base64,{}", BASE64.encode(svg)))
}
