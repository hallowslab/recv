use recv::qr;

#[test]
fn svg_data_url_has_prefix() {
    let url = qr::render_svg_data_url("http://192.168.1.10:41213/#t=abc").unwrap();
    assert!(url.starts_with("data:image/svg+xml;base64,"));
}

#[test]
fn terminal_render_is_not_empty() {
    let rendered = qr::render_terminal("http://192.168.1.10:41213/#t=abc").unwrap();
    assert!(rendered.lines().count() > 4);
}
