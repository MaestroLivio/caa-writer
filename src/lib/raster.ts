// Rasterizzazione SVG -> PNG nel WebView.
//
// Serve solo per l'esportazione DOCX: Word non gestisce in modo affidabile
// gli SVG incorporati. Il formato nativo `.caadoc` conserva sempre l'SVG.
// Se la rasterizzazione fallisce, l'esportazione prosegue e segnala l'avviso.

const cache = new Map<string, string>();

export async function svgToPngBase64(svg: string, size = 256): Promise<string | null> {
  const key = `${size}:${svg.length}:${svg.slice(0, 64)}`;
  const cached = cache.get(key);
  if (cached) return cached;

  try {
    const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const image = new Image();
    image.decoding = "sync";
    const loaded = new Promise<boolean>((resolve) => {
      const timer = setTimeout(() => resolve(false), 4000);
      image.onload = () => {
        clearTimeout(timer);
        resolve(true);
      };
      image.onerror = () => {
        clearTimeout(timer);
        resolve(false);
      };
    });
    image.src = url;
    const ok = await loaded;
    if (!ok) {
      URL.revokeObjectURL(url);
      return null;
    }

    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      URL.revokeObjectURL(url);
      return null;
    }
    ctx.clearRect(0, 0, size, size);
    ctx.drawImage(image, 0, 0, size, size);
    URL.revokeObjectURL(url);

    const dataUrl = canvas.toDataURL("image/png");
    const base64 = dataUrl.split(",", 2)[1] ?? null;
    if (base64) cache.set(key, base64);
    return base64;
  } catch {
    return null;
  }
}

export async function rasterizeSymbols(
  symbols: Map<string, string>
): Promise<{ pngs: Map<string, string>; failed: string[] }> {
  const pngs = new Map<string, string>();
  const failed: string[] = [];
  for (const [symbolId, svg] of symbols) {
    const png = await svgToPngBase64(svg);
    if (png) pngs.set(symbolId, png);
    else failed.push(symbolId);
  }
  return { pngs, failed };
}
