use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=frontend/atlas-lockup-light.png");
    if env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }

    if let Err(error) = embed_windows_resources() {
        panic!("failed to embed Atlas Windows resources: {error}");
    }
}

fn embed_windows_resources() -> io::Result<()> {
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is available"));
    let icon_path = output_dir.join("atlas.ico");
    write_atlas_icon(&icon_path)?;

    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(icon_path.to_string_lossy().as_ref())
        .set("ProductName", "Atlas")
        .set("FileDescription", "Atlas Desktop IDE")
        .set("InternalName", "Atlas")
        .set("OriginalFilename", "Atlas.exe");
    resource.compile().map_err(io::Error::other)
}

fn write_atlas_icon(path: &Path) -> io::Result<()> {
    const SIZE: u32 = 64;
    let source = fs::read("frontend/atlas-lockup-light.png")?;
    let decoder = png::Decoder::new(source.as_slice());
    let mut reader = decoder.read_info().map_err(io::Error::other)?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(io::Error::other)?;
    let rgba = rgba_pixels(&buffer[..info.buffer_size()], info.color_type);
    let scaled = scale_rgba_fit(&rgba, info.width, info.height, SIZE, SIZE);
    let mut pixels = vec![0u8; scaled.len()];
    for y in 0..SIZE {
        let source_row =
            &scaled[((SIZE - 1 - y) * SIZE * 4) as usize..((SIZE - y) * SIZE * 4) as usize];
        let target = (y * SIZE * 4) as usize;
        for x in 0..SIZE as usize {
            let s = x * 4;
            pixels[target + s..target + s + 4].copy_from_slice(&[
                source_row[s + 2],
                source_row[s + 1],
                source_row[s],
                source_row[s + 3],
            ]);
        }
    }

    let mask_stride = ((SIZE + 31) / 32) * 4;
    let mask = vec![0u8; (mask_stride * SIZE) as usize];
    let image_size = 40 + pixels.len() + mask.len();
    let image_offset = 6 + 16;
    let mut ico = Vec::with_capacity(image_offset + image_size);
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&[SIZE as u8, SIZE as u8, 0, 0]);
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&(image_size as u32).to_le_bytes());
    ico.extend_from_slice(&(image_offset as u32).to_le_bytes());
    ico.extend_from_slice(&40u32.to_le_bytes());
    ico.extend_from_slice(&(SIZE as i32).to_le_bytes());
    ico.extend_from_slice(&((SIZE * 2) as i32).to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&32u16.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&((pixels.len() + mask.len()) as u32).to_le_bytes());
    ico.extend_from_slice(&0i32.to_le_bytes());
    ico.extend_from_slice(&0i32.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&0u32.to_le_bytes());
    ico.extend_from_slice(&pixels);
    ico.extend_from_slice(&mask);
    fs::write(path, ico)
}

fn rgba_pixels(bytes: &[u8], color: png::ColorType) -> Vec<u8> {
    match color {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|v| [*v, *v, *v, 255]).collect(),
        png::ColorType::Indexed => Vec::new(),
    }
}
fn scale_rgba_fit(source: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0; (dw * dh * 4) as usize];
    let scale = (dw as f32 / sw as f32).min(dh as f32 / sh as f32);
    let rw = (sw as f32 * scale).round() as u32;
    let rh = (sh as f32 * scale).round() as u32;
    let ox = (dw - rw) / 2;
    let oy = (dh - rh) / 2;
    for y in 0..rh {
        for x in 0..rw {
            let sx = (x as u64 * sw as u64 / rw as u64) as u32;
            let sy = (y as u64 * sh as u64 / rh as u64) as u32;
            let s = ((sy * sw + sx) * 4) as usize;
            let d = (((oy + y) * dw + ox + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&source[s..s + 4]);
        }
    }
    out
}
