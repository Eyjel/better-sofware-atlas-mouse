#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hid;
mod model;
mod storage;
mod ui;

use eframe::egui;

pub const ATLAS_ICO_BYTES: &[u8] = include_bytes!("../assets/Icon.ico");

fn main() -> eframe::Result<()> {
    let icon = decode_ico_to_icon_data(ATLAS_ICO_BYTES, 64).unwrap_or_else(fallback_icon);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Press Play ATLAS — Control Center")
            .with_inner_size([1180.0, 750.0])
            .with_min_inner_size([980.0, 650.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Press Play ATLAS",
        native_options,
        Box::new(|cc| Ok(Box::new(ui::app::AtlasApp::new(cc)))),
    )
}

/// Decodes a 32-bit BGRA DIB entry from a Windows `.ico` file into top-down RGBA pixels.
pub fn decode_ico_rgba(ico: &[u8], preferred_size: u32) -> Option<(Vec<u8>, u32, u32)> {
    if ico.len() < 6 {
        return None;
    }
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    if count == 0 || ico.len() < 6 + count * 16 {
        return None;
    }

    // Pick entry matching preferred_size, or closest >= 32px
    let mut best: Option<(u32, u32, usize, usize)> = None;
    for i in 0..count {
        let o = 6 + i * 16;
        let w = if ico[o] == 0 { 256 } else { ico[o] as u32 };
        let h = if ico[o + 1] == 0 { 256 } else { ico[o + 1] as u32 };
        let size = u32::from_le_bytes([ico[o + 8], ico[o + 9], ico[o + 10], ico[o + 11]]) as usize;
        let offset =
            u32::from_le_bytes([ico[o + 12], ico[o + 13], ico[o + 14], ico[o + 15]]) as usize;

        if offset + size > ico.len() || size < 40 {
            continue;
        }
        // Only handle uncompressed 32-bit BITMAPINFOHEADER DIBs (header size == 40)
        let hdr_size = u32::from_le_bytes([
            ico[offset],
            ico[offset + 1],
            ico[offset + 2],
            ico[offset + 3],
        ]);
        let bpp = u16::from_le_bytes([ico[offset + 14], ico[offset + 15]]);
        if hdr_size != 40 || bpp != 32 {
            continue;
        }

        let is_better = match best {
            None => true,
            Some((bw, _, _, _)) => {
                let d_new = w.abs_diff(preferred_size);
                let d_old = bw.abs_diff(preferred_size);
                d_new < d_old
            }
        };
        if is_better {
            best = Some((w, h, offset, size));
        }
    }

    let (w, h, offset, _) = best?;
    let pixel_start = offset + 40;
    let pixel_bytes = (w * h * 4) as usize;
    if pixel_start + pixel_bytes > ico.len() {
        return None;
    }

    let src = &ico[pixel_start..pixel_start + pixel_bytes];
    let mut rgba = vec![0u8; pixel_bytes];
    // ICO DIB pixel rows are stored bottom-up in BGRA order
    let row_stride = (w * 4) as usize;
    for y in 0..(h as usize) {
        let src_row = (h as usize - 1 - y) * row_stride;
        let dst_row = y * row_stride;
        for x in 0..(w as usize) {
            let si = src_row + x * 4;
            let di = dst_row + x * 4;
            rgba[di] = src[si + 2];     // R
            rgba[di + 1] = src[si + 1]; // G
            rgba[di + 2] = src[si];     // B
            rgba[di + 3] = src[si + 3]; // A
        }
    }

    Some((rgba, w, h))
}

fn decode_ico_to_icon_data(ico: &[u8], preferred_size: u32) -> Option<egui::IconData> {
    let (rgba, width, height) = decode_ico_rgba(ico, preferred_size)?;
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

fn fallback_icon() -> egui::IconData {
    egui::IconData {
        rgba: vec![0, 214, 255, 255].repeat(16 * 16),
        width: 16,
        height: 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_ico_decodes_cleanly() {
        let (rgba, w, h) = decode_ico_rgba(ATLAS_ICO_BYTES, 64).expect("Icon.ico should decode");
        assert_eq!((w, h), (64, 64));
        assert_eq!(rgba.len(), 64 * 64 * 4);
        let visible_pixels = rgba.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(visible_pixels > 500, "Icon.ico must contain visible pixels");
    }
}
