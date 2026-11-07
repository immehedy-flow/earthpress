export type ImageFormat = 'jpg' | 'png' | 'webp' | 'avif';

export type SizeMode = 'original' | 'resize' | 'aspect' | 'custom';

/** Level: one quality for the batch. Target: highest quality that fits a byte target. */
export type QualityMode = 'level' | 'target';

export type ResizeKind = 'width' | 'height' | 'max' | 'percent';

export type AspectPreset = '1:1' | '4:5' | '5:4' | '4:3' | '3:2' | '16:9' | '9:16';

export type FitMode = 'cover' | 'contain' | 'fill' | 'inside' | 'outside';

export type CropPosition = 'center' | 'top' | 'right' | 'bottom' | 'left' | 'attention' | 'entropy';

export type SortField = 'added' | 'name' | 'size' | 'width' | 'height' | 'format';

export type SortOrder = 'asc' | 'desc';

/** What the user picked for the theme control. */
export type ThemePreference = 'light' | 'dark' | 'system';

/** What the preference resolves to once the OS is taken into account. */
export type ResolvedTheme = 'light' | 'dark';

/** The value handed to the native window chrome. */
export type ThemeMode = ResolvedTheme;

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
  /** Whether `quality` or `targetBytes` decides the encoder's quality. */
  qualityMode: QualityMode;
  /** Target output size in bytes, used when `qualityMode` is 'target'. */
  targetBytes: number;
  sizeMode: SizeMode;
  resizeKind: ResizeKind;
  resizeValue: number;
  aspectPreset: AspectPreset;
  customWidth: number;
  customHeight: number;
  fitMode: FitMode;
  cropPosition: CropPosition;
  background: string;
  pngCompression: number;
  pngPalette: boolean;
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
  /** The quality the encoder used. Differs from the slider in target mode. */
  qualityUsed?: number;
  error?: string;
};

export type ConversionProgress = {
  id: string;
  name: string;
  progress: number; // 0-100
};

/** Per-image "what will this cost" answer from the sample encode. */
export type ImageEstimate = {
  id: string;
  outputWidth: number;
  outputHeight: number;
  estimatedBytes: number;
  /** The quality the encoder settled on (differs from the slider in target-size mode). */
  qualityUsed: number;
  /** False when even the lowest quality overshoots the requested target size. */
  meetsTarget: boolean;
};

/** Per-image target geometry for the live dimension readout. */
export type ImageTarget = {
  id: string;
  width: number;
  height: number;
};

/** What the selected image's before/after preview card needs. */
export type PreviewPayload = {
  beforeDataUrl: string;
  afterDataUrl: string;
  outputWidth: number;
  outputHeight: number;
  qualityUsed: number;
};

export type Preset = {
  name: string;
  options: Partial<ConvertOptions>;
};

export type DragDropHandlers = {
  /** Called with absolute file paths whenever the OS drops files on the window. */
  onDrop: (paths: string[]) => void;
  /** Called when the OS drag enters or leaves the window. */
  onHover: (hovering: boolean) => void;
};

declare global {
  interface Window {
    earthpress: {
      selectImages: () => Promise<ImageItem[]>;
      inspectFiles: (filePaths: string[]) => Promise<ImageItem[]>;
      selectOutputDir: () => Promise<string | null>;
      convertImages: (files: ImageItem[], options: ConvertOptions) => Promise<ConvertResult[]>;
      /** Both halves of the preview card for one file, at display size. */
      previewPair: (filePath: string, options: ConvertOptions) => Promise<PreviewPayload>;
      /**
       * What every file will weigh with these settings.
       *
       * Unreadable files are left out rather than failing the whole pass, so
       * callers must tolerate a shorter array than they sent.
       */
      estimateImages: (
        inputs: Array<Pick<ImageItem, 'id' | 'path' | 'width' | 'height'>>,
        options: ConvertOptions,
      ) => Promise<ImageEstimate[]>;
      /** OS notification for a finished batch, skipped while the window is focused. */
      notifyBatchFinished: (title: string, body: string) => Promise<void>;
      revealPath: (filePath: string) => Promise<boolean>;
      onConversionProgress: (callback: (progress: ConversionProgress) => void) => () => void;
      cancelConversion: () => Promise<void>;
      /** Registers an OS-level file drag/drop listener. Returns an unsubscribe function. */
      subscribeDragDrop: (handlers: DragDropHandlers) => () => void;
      /** Syncs the native window chrome with the in-app theme. */
      setNativeTheme: (theme: ThemeMode) => Promise<void>;
    };
  }
}
