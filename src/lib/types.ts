export interface BaseFile {
  path: string;
  name: string;
  status: string;
}

export const IMAGE_EXTENSIONS = [
  "jpg", "jpeg", "png", "webp", "heic", "heif", "tiff", "bmp", "avif", "jxl",
] as const;

export const IMAGE_EXTENSIONS_RE = /\.(jpe?g|png|webp|heic|heif|tiff?|bmp|avif|jxl)$/i;

export const VIDEO_EXTENSIONS = [
  "mp4", "mov", "avi", "mkv", "webm", "m4v", "3gp",
] as const;

export const VIDEO_EXTENSIONS_RE = /\.(mp4|mov|avi|mkv|webm|m4v|3gp)$/i;
