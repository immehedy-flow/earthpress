export type ImageFormat = 'jpg' | 'png' | 'webp' | 'avif';

export type ResizeMode = 'original' | 'width' | 'height' | 'max' | 'percent';

export type SizeMode = 'original' | 'resize' | 'aspect' | 'custom';

export type ResizeKind = 'width' | 'height' | 'max' | 'percent';

export type AspectPreset = '1:1' | '4:5' | '5:4' | '4:3' | '3:2' | '16:9' | '9:16';

export type FitMode = 'cover' | 'contain' | 'fill' | 'inside' | 'outside';

export type CropPosition = 'center' | 'top' | 'right' | 'bottom' | 'left' | 'attention' | 'entropy';

export type ImageItem = {
  id: string;
  path: string;
  name: string;
  directory: string;
  format: string;
  width: number;
  height: number;
  size: number;
  preview: string;
  error?: string;
};

export type ConvertOptions = {
  format: ImageFormat;
  quality: number;
  lossless: boolean;
  sizeMode: SizeMode;
  resizeKind: ResizeKind;
  resizeMode: ResizeMode;
  resizeValue: number;
  aspectPreset: AspectPreset;
  customWidth: number;
  customHeight: number;
  fitMode: FitMode;
  cropPosition: CropPosition;
  background: string;
  allowUpscale: boolean;
  pngCompression: number;
  pngPalette: boolean;
  preserveMetadata: boolean;
  overwrite: boolean;
  filenameSuffix: string;
  outputDir: string;
};

export type ConvertResult = {
  id: string;
  name: string;
  ok: boolean;
  outputPath?: string;
  outputSize?: number;
  savedBytes?: number;
  error?: string;
};

declare global {
  interface Window {
    earthpress: {
      selectImages: () => Promise<ImageItem[]>;
      inspectFiles: (filePaths: string[]) => Promise<ImageItem[]>;
      selectOutputDir: () => Promise<string | null>;
      convertImages: (files: ImageItem[], options: ConvertOptions) => Promise<ConvertResult[]>;
      revealPath: (filePath: string) => Promise<boolean>;
      getPathForFile: (file: File) => string;
    };
  }
}
