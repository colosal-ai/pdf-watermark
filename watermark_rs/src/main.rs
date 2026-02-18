mod pdf;
mod watermark;
mod builder;

use clap::Parser;
use anyhow::Result;

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
}

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
    let wm = watermark::prepare(&args.logo)?;

    println!("[3/4] Aplicando marca de agua...");
    let total = pages.len();
    let result: Vec<_> = pages
        .into_iter()
        .enumerate()
        .map(|(i, page)| {
            let img = watermark::apply(&page, &wm);
            println!("  Página {}/{} ✓", i + 1, total);
            img
        })
        .collect();

    println!("[4/4] Reconstruyendo PDF...");
    builder::build_pdf(&result, &args.output, &quality)?;

    println!("Listo.");
    Ok(())
}
