#!/usr/bin/env python3
"""
Extrae imágenes de un PDF, aplica marca de agua y reconstruye el PDF.

Uso:
  python3 watermark.py [--logo RUTA] [--quality CALIDAD] [--output NOMBRE]

  --logo     Ruta a la imagen de marca de agua (default: sources/logo_colosal.png)
  --quality  Calidad JPEG 1-100 o "lossless" para PNG sin pérdida (default: lossless)
  --output   Nombre del archivo PDF de salida (default: output_watermarked.pdf)
             Valores orientativos: 95=alta, 85=buena, 75=media
"""

import argparse
import subprocess
import sys
import os
from pathlib import Path
from PIL import Image
import img2pdf

# --- CONFIGURACIÓN ---
PDF_INPUT   = "sources/example1.pdf"
PDF_OUTPUT  = "output_watermarked.pdf"
TMP_DIR     = "tmp_pages"

PAGE_W, PAGE_H = 1376, 768

WM_MAX_W = 120
WM_MIN_W = 107
WM_MIN_H = 21
WM_OPACITY = 1.0           # 0.0 = invisible, 1.0 = opaco
WM_MARGIN  = 0             # px desde el borde


def parse_args():
    parser = argparse.ArgumentParser(
        description="Aplica marca de agua a un PDF de presentación."
    )
    parser.add_argument(
        "--logo",
        default="sources/logo_colosal.png",
        help="Ruta a la imagen de marca de agua (default: sources/logo_colosal.png)"
    )
    parser.add_argument(
        "--quality",
        default="lossless",
        help="Calidad: 'lossless' (PNG, sin pérdida) o 1-100 (JPEG). Default: lossless"
    )
    parser.add_argument(
        "--output", "-o",
        default="output_watermarked.pdf",
        help="Nombre del archivo PDF de salida (default: output_watermarked.pdf)"
    )
    return parser.parse_args()


def calc_watermark_size(orig_w, orig_h):
    """Calcula tamaño de marca de agua respetando restricciones."""
    ratio = orig_h / orig_w

    new_w = WM_MAX_W
    new_h = int(round(new_w * ratio))

    if new_h < WM_MIN_H:
        new_h = WM_MIN_H
        new_w = int(round(new_h / ratio))

    if new_w < WM_MIN_W:
        new_w = WM_MIN_W
        new_h = int(round(new_w * ratio))

    return new_w, new_h


def extract_pages(pdf_path, tmp_dir):
    """Extrae cada página del PDF como PNG usando pdftoppm."""
    os.makedirs(tmp_dir, exist_ok=True)
    prefix = os.path.join(tmp_dir, "page")
    subprocess.run([
        "pdftoppm", "-png",
        "-r", "72",
        pdf_path, prefix
    ], check=True)
    pages = sorted(Path(tmp_dir).glob("page-*.png"))
    print(f"  Extraídas {len(pages)} páginas")
    return pages


def prepare_watermark(logo_path, opacity):
    """Carga, redimensiona y ajusta opacidad de la marca de agua."""
    logo = Image.open(logo_path).convert("RGBA")
    orig_w, orig_h = logo.size
    new_w, new_h = calc_watermark_size(orig_w, orig_h)
    logo = logo.resize((new_w, new_h), Image.LANCZOS)
    print(f"  Logo: {orig_w}x{orig_h} → {new_w}x{new_h}")

    r, g, b, a = logo.split()
    a = a.point(lambda p: int(p * opacity))
    logo = Image.merge("RGBA", (r, g, b, a))
    return logo


def apply_watermark(page_path, watermark):
    """Aplica marca de agua a una imagen y devuelve la imagen resultante."""
    page = Image.open(page_path).convert("RGBA")
    wm_w, wm_h = watermark.size

    x = page.width  - wm_w - WM_MARGIN
    y = page.height - wm_h - WM_MARGIN

    page.paste(watermark, (x, y), watermark)
    return page.convert("RGB")


def build_pdf(images, output_path, tmp_dir, quality):
    """Genera PDF. Lossless=PNG embebido, numérico=JPEG a esa calidad."""
    if not images:
        print("ERROR: Sin imágenes para generar PDF")
        sys.exit(1)

    lossless = (quality == "lossless")
    ext = "png" if lossless else "jpg"
    img_paths = []

    for i, img in enumerate(images):
        p = os.path.join(tmp_dir, f"wm_{i:03d}.{ext}")
        if lossless:
            img.save(p, "PNG", optimize=False)
        else:
            img.save(p, "JPEG", quality=int(quality), subsampling=0)
        img_paths.append(p)

    layout = img2pdf.get_fixed_dpi_layout_fun((72, 72))
    with open(output_path, "wb") as f:
        f.write(img2pdf.convert(img_paths, layout_fun=layout))

    size_mb = os.path.getsize(output_path) / (1024 * 1024)
    mode = "PNG lossless" if lossless else f"JPEG q={quality}"
    print(f"  PDF generado: {output_path} ({size_mb:.1f} MB, {mode})")


def main():
    args = parse_args()
    base = Path(__file__).resolve().parent
    os.chdir(base)

    if not os.path.isfile(args.logo):
        print(f"ERROR: Logo no encontrado: {args.logo}")
        sys.exit(1)

    if args.quality != "lossless":
        try:
            q = int(args.quality)
            if not 1 <= q <= 100:
                raise ValueError
        except ValueError:
            print("ERROR: --quality debe ser 'lossless' o un número 1-100")
            sys.exit(1)

    print(f"  Logo:    {args.logo}")
    print(f"  Calidad: {args.quality}")
    print(f"  Salida:  {args.output}")
    print()

    print("[1/4] Extrayendo páginas del PDF...")
    pages = extract_pages(PDF_INPUT, TMP_DIR)

    print("[2/4] Preparando marca de agua...")
    watermark = prepare_watermark(args.logo, WM_OPACITY)

    print("[3/4] Aplicando marca de agua...")
    result_images = []
    for i, p in enumerate(pages):
        img = apply_watermark(p, watermark)
        result_images.append(img)
        print(f"  Página {i+1}/{len(pages)} ✓")

    print("[4/4] Reconstruyendo PDF...")
    build_pdf(result_images, args.output, TMP_DIR, args.quality)

    for p in Path(TMP_DIR).glob("*"):
        p.unlink()
    Path(TMP_DIR).rmdir()
    print("Listo. Temporales eliminados.")


if __name__ == "__main__":
    main()
