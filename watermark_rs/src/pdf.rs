use anyhow::{anyhow, Context, Result};
use flate2::read::ZlibDecoder;
use image::{DynamicImage, RgbImage};
use lopdf::{Document, Object};
use std::io::{Cursor, Read};

pub fn extract_pages(path: &str) -> Result<Vec<DynamicImage>> {
    let doc = Document::load(path).context("No se pudo abrir el PDF")?;
    let mut images = Vec::new();

    let mut page_ids: Vec<_> = doc.get_pages().into_iter().collect();
    page_ids.sort_by_key(|(num, _)| *num);

    for (page_num, page_id) in &page_ids {
        let image = extract_page_image(&doc, *page_id)
            .with_context(|| format!("Error en página {}", page_num))?;
        images.push(image);
    }

    Ok(images)
}

fn extract_page_image(doc: &Document, page_id: lopdf::ObjectId) -> Result<DynamicImage> {
    let page_dict = doc
        .get_object(page_id)?
        .as_dict()
        .map_err(|_| anyhow!("Página no es un diccionario"))?;

    let resources = resolve_to_dict(doc, page_dict.get(b"Resources")?)?;
    let xobjects = resolve_to_dict(doc, resources.get(b"XObject")?)?;

    for (name, obj_ref) in xobjects.iter() {
        let object = resolve(doc, obj_ref)?;

        if let Object::Stream(ref stream) = object {
            let dict = &stream.dict;

            if !is_name(dict, b"Subtype", "Image") {
                continue;
            }
            if !is_name(dict, b"ColorSpace", "DeviceRGB") {
                continue;
            }

            let width = get_uint(dict, b"Width")?;
            let height = get_uint(dict, b"Height")?;

            let _name = String::from_utf8_lossy(name);
            return decode_stream(stream, width, height);
        }
    }

    Err(anyhow!("No se encontró imagen RGB en la página"))
}

fn decode_stream(stream: &lopdf::Stream, w: u32, h: u32) -> Result<DynamicImage> {
    let filter = stream
        .dict
        .get(b"Filter")
        .ok()
        .and_then(|f| f.as_name_str().ok())
        .unwrap_or("");

    match filter {
        "FlateDecode" => {
            let mut decoder = ZlibDecoder::new(&stream.content[..]);
            let mut data = Vec::new();
            decoder
                .read_to_end(&mut data)
                .context("Error descomprimiendo FlateDecode")?;

            let components: u32 = 3;
            let expected_raw = (w * h * components) as usize;

            let expected_png = ((w * components + 1) * h) as usize;
            let data = if data.len() == expected_raw {
                data
            } else if data.len() == expected_png {
                remove_png_predictor(&data, w, components)
            } else {
                data
            };

            if data.len() != expected_raw {
                return Err(anyhow!(
                    "Tamaño inesperado: {} bytes (esperados {})",
                    data.len(),
                    expected_raw
                ));
            }
            let rgb = RgbImage::from_raw(w, h, data)
                .ok_or_else(|| anyhow!("Datos de imagen inválidos"))?;
            Ok(DynamicImage::ImageRgb8(rgb))
        }
        "DCTDecode" => {
            let cursor = Cursor::new(&stream.content);
            let img = image::load(cursor, image::ImageFormat::Jpeg)?;
            Ok(img)
        }
        "" => {
            let rgb = RgbImage::from_raw(w, h, stream.content.clone())
                .ok_or_else(|| anyhow!("Datos de imagen inválidos (sin filtro)"))?;
            Ok(DynamicImage::ImageRgb8(rgb))
        }
        other => Err(anyhow!("Filtro no soportado: {}", other)),
    }
}

fn resolve(doc: &Document, obj: &Object) -> Result<Object> {
    match obj {
        Object::Reference(id) => doc
            .get_object(*id)
            .map(|o| o.clone())
            .map_err(|e| anyhow!("Referencia {:?} no encontrada: {}", id, e)),
        other => Ok(other.clone()),
    }
}

fn resolve_to_dict(doc: &Document, obj: &Object) -> Result<lopdf::Dictionary> {
    let resolved = resolve(doc, obj)?;
    match resolved {
        Object::Dictionary(d) => Ok(d),
        Object::Stream(s) => Ok(s.dict),
        other => Err(anyhow!("Se esperaba diccionario, encontrado: {:?}", other)),
    }
}

fn is_name(dict: &lopdf::Dictionary, key: &[u8], expected: &str) -> bool {
    dict.get(key)
        .ok()
        .and_then(|v| v.as_name_str().ok())
        .map(|s| s == expected)
        .unwrap_or(false)
}

fn get_uint(dict: &lopdf::Dictionary, key: &[u8]) -> Result<u32> {
    let val = dict.get(key)?;
    val.as_i64()
        .map(|v| v as u32)
        .map_err(|_| anyhow!("Se esperaba entero para {:?}", std::str::from_utf8(key)))
}

fn remove_png_predictor(data: &[u8], width: u32, components: u32) -> Vec<u8> {
    let stride = (width * components) as usize;
    let row_len = stride + 1;
    let rows = data.len() / row_len;
    let comp = components as usize;

    let mut result = Vec::with_capacity(stride * rows);
    let mut prev_row = vec![0u8; stride];

    for r in 0..rows {
        let row = &data[r * row_len..r * row_len + row_len];
        let filter = row[0];
        let raw = &row[1..];
        let mut decoded = vec![0u8; stride];

        match filter {
            0 => decoded.copy_from_slice(raw),
            1 => {
                for i in 0..stride {
                    let a = if i >= comp { decoded[i - comp] } else { 0 };
                    decoded[i] = raw[i].wrapping_add(a);
                }
            }
            2 => {
                for i in 0..stride {
                    decoded[i] = raw[i].wrapping_add(prev_row[i]);
                }
            }
            3 => {
                for i in 0..stride {
                    let a = if i >= comp { decoded[i - comp] as u16 } else { 0 };
                    let b = prev_row[i] as u16;
                    decoded[i] = raw[i].wrapping_add(((a + b) / 2) as u8);
                }
            }
            4 => {
                for i in 0..stride {
                    let a = if i >= comp { decoded[i - comp] } else { 0 };
                    let b = prev_row[i];
                    let c = if i >= comp { prev_row[i - comp] } else { 0 };
                    decoded[i] = raw[i].wrapping_add(paeth(a, b, c));
                }
            }
            _ => decoded.copy_from_slice(raw),
        }

        result.extend_from_slice(&decoded);
        prev_row = decoded;
    }

    result
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
