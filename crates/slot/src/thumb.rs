use slot_retro::FrameGeometry;

pub fn png(xrgb8888: &[u8]) -> Option<Vec<u8>> {
    png_frame(xrgb8888, FrameGeometry::default())
}

pub fn png_frame(xrgb8888: &[u8], geometry: FrameGeometry) -> Option<Vec<u8>> {
    let FrameGeometry { width, height, .. } = geometry;
    if !(1..=2048).contains(&width) || !(1..=2048).contains(&height) {
        return None;
    }
    let n = (width * height) as usize;
    if xrgb8888.len() < n * 4 {
        return None;
    }
    let mut rgb = Vec::with_capacity(n * 3);
    for px in xrgb8888[..n * 4].chunks_exact(4) {
        rgb.extend_from_slice(&[px[2], px[1], px[0]]);
    }

    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, width, height);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().ok()?;
    writer.write_image_data(&rgb).ok()?;
    writer.finish().ok()?;
    Some(out)
}
