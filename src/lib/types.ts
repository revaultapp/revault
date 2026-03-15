export interface BaseFile {
  path: string;
  name: string;
  status: string;
}

export const IMAGE_EXTENSIONS = [
  "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "bmp", "gif", "jxl",
] as const;

export const IMAGE_EXTENSIONS_RE = /\.(jpe?g|png|webp|heic|heif|tiff?|bmp|gif|jxl)$/i;
