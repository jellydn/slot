use std::io::Read;
use std::path::Path;

const MAX_CART_BYTES: u64 = 2 * 1024 * 1024;
const SIDE: usize = 128;

// PICO-8's 16 base colours followed by the 16 extended label colours (g-v).
const PALETTE: [[u8; 3]; 32] = [
    [0, 0, 0],
    [29, 43, 83],
    [126, 37, 83],
    [0, 135, 81],
    [171, 82, 54],
    [95, 87, 79],
    [194, 195, 199],
    [255, 241, 232],
    [255, 0, 77],
    [255, 163, 0],
    [255, 236, 39],
    [0, 228, 54],
    [41, 173, 255],
    [131, 118, 156],
    [255, 119, 168],
    [255, 204, 170],
    [41, 24, 20],
    [17, 29, 53],
    [66, 33, 54],
    [18, 83, 89],
    [116, 47, 41],
    [73, 51, 59],
    [162, 136, 121],
    [243, 239, 125],
    [190, 18, 80],
    [255, 108, 36],
    [168, 231, 46],
    [0, 181, 67],
    [6, 90, 181],
    [117, 70, 101],
    [255, 110, 89],
    [255, 157, 129],
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelError {
    Missing,
    Malformed,
    Unsupported,
    TooLarge,
    Read,
    Write,
}

/// Read only local label pixels. Never run Lua, follow includes or fetch a cart.
pub fn read_label(path: &Path) -> Result<Vec<u8>, LabelError> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(LabelError::Unsupported)?;
    let name = name.to_ascii_lowercase();
    let is_png = name.ends_with(".p8.png");
    if !is_png && !name.ends_with(".p8") {
        return Err(LabelError::Unsupported);
    }
    let file = std::fs::File::open(path).map_err(|_| LabelError::Read)?;
    let mut bytes = Vec::new();
    file.take(MAX_CART_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LabelError::Read)?;
    if bytes.len() as u64 > MAX_CART_BYTES {
        return Err(LabelError::TooLarge);
    }
    // RomM and fake-08 also accept PNG-encoded cartridges named .p8.
    let pixels = if is_png || bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        png_label(&bytes)?
    } else {
        text_label(&bytes)?
    };
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, SIDE as u32, SIDE as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .map_err(|_| LabelError::Malformed)?
            .write_image_data(&pixels)
            .map_err(|_| LabelError::Malformed)?;
    }
    Ok(out)
}

fn text_label(bytes: &[u8]) -> Result<Vec<u8>, LabelError> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let mut lines = bytes
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l));
    if !lines
        .next()
        .is_some_and(|l| l.starts_with(b"pico-8 cartridge //"))
    {
        return Err(LabelError::Malformed);
    }
    let version = lines
        .next()
        .and_then(|l| l.strip_prefix(b"version "))
        .ok_or(LabelError::Malformed)?;
    if version.is_empty() || !version.iter().all(u8::is_ascii_digit) {
        return Err(LabelError::Malformed);
    }
    let (mut found, mut inside) = (false, false);
    let mut pixels = Vec::with_capacity(SIDE * SIDE * 3);
    let mut rows = 0;
    for line in lines {
        if line.starts_with(b"__") && line.ends_with(b"__") {
            inside = line == b"__label__";
            if inside && found {
                return Err(LabelError::Malformed);
            }
            found |= inside;
            continue;
        }
        if !inside || line.is_empty() {
            continue;
        }
        if rows == SIDE || line.len() != SIDE {
            return Err(LabelError::Malformed);
        }
        for &c in line {
            let index = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'v' => c - b'a' + 10,
                _ => return Err(LabelError::Malformed),
            };
            pixels.extend_from_slice(&PALETTE[index as usize]);
        }
        rows += 1;
    }
    if !found {
        return Err(LabelError::Missing);
    }
    if rows != SIDE {
        return Err(LabelError::Malformed);
    }
    Ok(pixels)
}

fn png_label(bytes: &[u8]) -> Result<Vec<u8>, LabelError> {
    let mut decoder = png::Decoder::new_with_limits(bytes, png::Limits { bytes: 1024 * 1024 });
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| LabelError::Malformed)?;
    let info = reader.info();
    if (info.width, info.height) != (160, 205) || info.animation_control.is_some() {
        return Err(LabelError::Unsupported);
    }
    let mut decoded = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut decoded)
        .map_err(|_| LabelError::Malformed)?;
    let channels = match frame.color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return Err(LabelError::Unsupported),
    };
    if frame.bit_depth != png::BitDepth::Eight {
        return Err(LabelError::Unsupported);
    }
    let mut pixels = Vec::with_capacity(SIDE * SIDE * 3);
    // Standard cartridge label rectangle, excluding the border/title and payload alpha.
    for y in 24..24 + SIDE {
        for x in 16..16 + SIDE {
            let at = (y * 160 + x) * channels;
            pixels.extend_from_slice(&decoded[at..at + 3]);
        }
    }
    // Validate the trailing PNG chunks too (including CRCs), not just its first frame.
    reader.finish().map_err(|_| LabelError::Malformed)?;
    Ok(pixels)
}
