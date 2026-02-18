use crate::watermark::Quality;
use anyhow::Result;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use ::image::DynamicImage;
use lopdf::{dictionary, Document, Object, Stream};
use std::io::Write;

const PAGE_W: f64 = 1376.0;
const PAGE_H: f64 = 768.0;

pub fn build_pdf(images: &[DynamicImage], output: &str, quality: &Quality) -> Result<()> {
    let mut doc = Document::with_version("1.4");

    let pages_id = doc.new_object_id();
    let mut page_ids: Vec<Object> = Vec::new();

    for img in images {
        let image_stream = encode_image_stream(img, quality)?;
        let img_id = doc.add_object(image_stream);

        let content = format!("q\n{} 0 0 {} 0 0 cm\n/Im0 Do\nQ\n", PAGE_W, PAGE_H);
        let content_stream = Stream::new(dictionary! {}, content.into_bytes());
        let content_id = doc.add_object(content_stream);

        let page = dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
            "Contents" => Object::Reference(content_id),
            "Resources" => dictionary! {
                "XObject" => dictionary! {
                    "Im0" => Object::Reference(img_id),
                },
            },
        };
        let page_id = doc.add_object(page);
        page_ids.push(Object::Reference(page_id));
    }

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids,
        "Count" => images.len() as i64,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(output)?;

    let size = std::fs::metadata(output)?.len();
    let mode = match quality {
        Quality::Lossless => "Flate lossless".to_string(),
        Quality::Jpeg(q) => format!("JPEG q={}", q),
    };
    println!(
        "  PDF generado: {} ({:.1} MB, {})",
        output,
        size as f64 / 1_048_576.0,
        mode
    );

    Ok(())
}

fn encode_image_stream(img: &DynamicImage, quality: &Quality) -> Result<Stream> {
    let rgb = img.to_rgb8();
    let (w, h) = ::image::GenericImageView::dimensions(&rgb);

    match quality {
        Quality::Lossless => {
            let raw = rgb.into_raw();
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&raw)?;
            let compressed = encoder.finish()?;

            let dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w as i64,
                "Height" => h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8_i64,
                "Filter" => "FlateDecode",
            };
            Ok(Stream::new(dict, compressed))
        }
        Quality::Jpeg(q) => {
            let mut buf: Vec<u8> = Vec::new();
            let encoder =
                ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, *q);
            img.write_with_encoder(encoder)?;

            let dict = dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w as i64,
                "Height" => h as i64,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8_i64,
                "Filter" => "DCTDecode",
            };
            Ok(Stream::new(dict, buf))
        }
    }
}
