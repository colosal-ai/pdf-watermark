pub mod pdf;
pub mod watermark;
pub mod builder;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn process_pdf(
    pdf_bytes: &[u8],
    logo_bytes: &[u8],
    quality_str: &str,
    page_indices: &[u32],
    position: &str,
    min_w: u32,
    min_h: u32,
) -> Result<Vec<u8>, JsValue> {
    let quality = watermark::parse_quality(quality_str)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let all_pages = pdf::extract_pages_from_bytes(pdf_bytes)
        .map_err(|e| JsValue::from_str(&format!("Error extrayendo páginas: {}", e)))?;

    let pages: Vec<_> = if page_indices.is_empty() {
        all_pages
    } else {
        page_indices
            .iter()
            .filter_map(|&i| all_pages.get(i as usize).cloned())
            .collect()
    };

    if pages.is_empty() {
        return Err(JsValue::from_str("No se seleccionaron páginas válidas"));
    }

    let pos = if position.is_empty() { "br" } else { position };

    let wm = watermark::prepare_from_bytes(logo_bytes, min_w, min_h)
        .map_err(|e| JsValue::from_str(&format!("Error preparando logo: {}", e)))?;

    let result: Vec<_> = pages.iter().map(|page| watermark::apply(page, &wm, pos)).collect();

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
