//! Reads text that can't be selected (images, videos, scanned PDFs) by
//! capturing a screen region and running Windows' built-in offline OCR.

use anyhow::{anyhow, Result};
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

use crate::text::is_arabic;

/// Captures a rectangle of the virtual desktop, in physical pixels, as BGRA
pub fn capture(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>> {
    if w <= 0 || h <= 0 {
        return Err(anyhow!("empty selection"));
    }
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bitmap.into());
        let blit = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                // Negative height gives top-down rows, which is what OCR expects
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let rows = GetDIBits(mem, bitmap, 0, h as u32, Some(pixels.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        blit?;
        if rows == 0 {
            return Err(anyhow!("screen capture failed"));
        }
        Ok(pixels)
    }
}

/// Small text OCRs much better when enlarged, so tiny regions are upscaled
fn upscale(pixels: &[u8], w: i32, h: i32, factor: i32) -> Vec<u8> {
    let nw = w * factor;
    let mut out = vec![0u8; (nw * h * factor * 4) as usize];
    for y in 0..h * factor {
        for x in 0..nw {
            let src = (((y / factor) * w + x / factor) * 4) as usize;
            let dst = ((y * nw + x) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
        }
    }
    out
}

fn recognize_blocking(pixels: Vec<u8>, w: i32, h: i32) -> Result<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let max = OcrEngine::MaxImageDimension()? as i32;
    let factor = if h < 400 && w * 2 <= max && h * 2 <= max { 2 } else { 1 };
    let (pixels, w, h) = if factor > 1 { (upscale(&pixels, w, h, factor), w * factor, h * factor) } else { (pixels, w, h) };

    let writer = DataWriter::new()?;
    writer.WriteBytes(&pixels)?;
    let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&writer.DetachBuffer()?, BitmapPixelFormat::Bgra8, w, h)?;

    // Run every installed OCR language and keep the result that found the most
    // characters of its own script, so Arabic and English both work unattended
    let mut best = (0usize, String::new());
    for lang in OcrEngine::AvailableRecognizerLanguages()? {
        let tag = lang.LanguageTag()?.to_string();
        let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) else { continue };
        let result = engine.RecognizeAsync(&bitmap)?.get()?;
        let text = result
            .Lines()?
            .into_iter()
            .filter_map(|l| l.Text().ok().map(|t| t.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        let score = if tag.starts_with("ar") {
            text.chars().filter(|c| is_arabic(*c)).count()
        } else {
            text.chars().filter(|c| c.is_ascii_alphanumeric()).count()
        };
        if score > best.0 {
            best = (score, text);
        }
    }
    if best.1.trim().is_empty() {
        return Err(anyhow!("no text found in the selected area"));
    }
    Ok(best.1)
}

pub fn recognize_region(x: i32, y: i32, w: i32, h: i32) -> Result<String> {
    let pixels = capture(x, y, w, h)?;
    // WinRT needs a COM-initialized thread of its own
    std::thread::spawn(move || recognize_blocking(pixels, w, h))
        .join()
        .map_err(|_| anyhow!("OCR crashed"))?
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "reads the live screen; run with --ignored"]
    fn recognizes_text_on_screen() {
        let langs: Vec<String> = windows::Media::Ocr::OcrEngine::AvailableRecognizerLanguages()
            .unwrap()
            .into_iter()
            .map(|l| l.LanguageTag().unwrap().to_string())
            .collect();
        println!("OCR languages: {langs:?}");
        match super::recognize_region(0, 0, 1200, 500) {
            Ok(t) => println!("OCR text: {}", t.chars().take(300).collect::<String>()),
            Err(e) => println!("OCR error: {e}"),
        }
    }
}
