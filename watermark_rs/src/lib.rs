pub mod pdf;
pub mod watermark;
pub mod builder;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn process_pdf(
    pdf_bytes: &[u8],
    logo_bytes: &[u8],
    quality_str: &str,
) -> Result<Vec<u8>, JsValue> {
    let quality = watermark::parse_quality(quality_str)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let pages = pdf::extract_pages_from_bytes(pdf_bytes)
        .map_err(|e| JsValue::from_str(&format!("Error extrayendo páginas: {}", e)))?;

    let wm = watermark::prepare_from_bytes(logo_bytes)
        .map_err(|e| JsValue::from_str(&format!("Error preparando logo: {}", e)))?;

    let result: Vec<_> = pages.iter().map(|page| watermark::apply(page, &wm)).collect();

    let pdf_out = builder::build_pdf_bytes(&result, &quality)
        .map_err(|e| JsValue::from_str(&format!("Error generando PDF: {}", e)))?;

    Ok(pdf_out)
}

#[wasm_bindgen]
pub fn get_page_count(pdf_bytes: &[u8]) -> Result<usize, JsValue> {
    let pages = pdf::extract_pages_from_bytes(pdf_bytes)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(pages.len())
}
