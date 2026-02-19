#[cfg(not(target_arch = "wasm32"))]
mod pdf;
#[cfg(not(target_arch = "wasm32"))]
mod watermark;
#[cfg(not(target_arch = "wasm32"))]
mod builder;

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(name = "watermark", about = "Aplica marca de agua a un PDF de presentación")]
struct Args {
    /// PDF de entrada
    input: String,

    /// Imagen de marca de agua (PNG o JPG)
    #[arg(long, default_value = "logo.png")]
    logo: String,

    /// Calidad: "lossless" o 1-100 (JPEG)
    #[arg(long, default_value = "lossless")]
    quality: String,

    /// Archivo PDF de salida
    #[arg(short, long, default_value = "output_watermarked.pdf")]
    output: String,

    /// Posición del watermark: tl,tc,tr,ml,mc,mr,bl,bc,br
    #[arg(long, default_value = "br")]
    position: String,

    /// Ancho mínimo del watermark
    #[arg(long, default_value = "107")]
    min_w: u32,

    /// Alto mínimo del watermark
    #[arg(long, default_value = "21")]
    min_h: u32,
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let args = Args::parse();
    let quality = watermark::parse_quality(&args.quality)?;

    println!("  Input:   {}", args.input);
    println!("  Logo:    {}", args.logo);
    println!("  Calidad: {}", args.quality);
    println!("  Salida:  {}", args.output);
    println!();

    println!("[1/4] Extrayendo páginas del PDF...");
    let pages = pdf::extract_pages(&args.input)?;
    println!("  Extraídas {} páginas", pages.len());

    println!("[2/4] Preparando marca de agua...");
    let wm = watermark::prepare(&args.logo, args.min_w, args.min_h)?;

    println!("[3/4] Aplicando marca de agua...");
    let total = pages.len();
    let result: Vec<_> = pages
        .into_iter()
        .enumerate()
        .map(|(i, page)| {
            let img = watermark::apply(&page, &wm, &args.position);
            println!("  Página {}/{} ✓", i + 1, total);
            img
        })
        .collect();

    println!("[4/4] Reconstruyendo PDF...");
    builder::build_pdf(&result, &args.output, &quality)?;

    println!("Listo.");
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
