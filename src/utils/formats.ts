import type { ConvertOptions, ImageFormat } from '../types'

type FormatInfo = {
  label: string
  hint: string
  /** The quality slider and level chips apply. */
  quality: boolean
  /** A lossless switch is offered. */
  lossless: boolean
  /** PNG-only: palette reduction and the compression slider. */
  palette: boolean
  compression: boolean
}

export const FORMATS: Array<{ value: ImageFormat; label: string }> = [
  { value: 'webp', label: 'WEBP' },
  { value: 'jpg', label: 'JPG' },
  { value: 'png', label: 'PNG' },
  { value: 'avif', label: 'AVIF' },
]

export const FORMAT_INFO: Record<ImageFormat, FormatInfo> = {
  webp: {
    label: 'WEBP',
    hint: 'WebP: small files, works almost everywhere. A good default.',
    quality: true,
    lossless: true,
    palette: false,
    compression: false,
  },
  jpg: {
    label: 'JPG',
    hint: 'JPG: universal, but no transparency and no lossless mode.',
    quality: true,
    lossless: false,
    palette: false,
    compression: false,
  },
  png: {
    label: 'PNG',
    hint: 'PNG: lossless with transparency; size comes from compression and palette.',
    quality: false,
    lossless: false,
    palette: true,
    compression: true,
  },
  avif: {
    label: 'AVIF',
    hint: 'AVIF: the smallest files, but slower to encode. Lossless is approximated at quality 100.',
    quality: true,
    lossless: true,
    palette: false,
    compression: false,
  },
}

export const QUALITY_LEVELS = [
  { value: 60, label: 'Web' },
  { value: 80, label: 'Balanced' },
  { value: 92, label: 'High' },
  { value: 100, label: 'Max' },
] as const

export function qualityName(quality: number) {
  return QUALITY_LEVELS.find((level) => level.value === quality)?.label ?? `Q${quality}`
}

export function formatLabel(format: ImageFormat) {
  return FORMAT_INFO[format].label
}

const SOURCE_LABELS: Record<string, string> = {
  webp: 'WebP',
  jpg: 'JPG',
  jpeg: 'JPG',
  png: 'PNG',
  avif: 'AVIF',
  tif: 'TIFF',
  tiff: 'TIFF',
  bmp: 'BMP',
}

/** How a source file's own format is written in the row meta. */
export function sourceLabel(format: string) {
  return SOURCE_LABELS[format] ?? format.toUpperCase()
}

/**
 * A plain-language summary of the size stage.
 *
 * Until the backend measures the batch this is what the readout can honestly
 * say; once dimensions are measured the card shows `source → target` instead.
 */
export function describeSize(options: ConvertOptions) {
  switch (options.sizeMode) {
    case 'original':
      return 'Keeps original dimensions'
    case 'resize':
      if (options.resizeKind === 'percent') {
        return `${options.resizeValue}% of the original`
      }
      if (options.resizeKind === 'width') {
        return `Width ${Math.round(options.resizeValue)} px`
      }
      if (options.resizeKind === 'height') {
        return `Height ${Math.round(options.resizeValue)} px`
      }
      return `Longest side ${Math.round(options.resizeValue)} px`
    case 'aspect':
      return `${options.aspectPreset} ratio`
    case 'custom':
      return `${Math.round(options.customWidth)} × ${Math.round(options.customHeight)} px`
  }
}

/** One line that describes a preset in the picker. */
export function describeOptions(options: ConvertOptions) {
  return `${formatLabel(options.format)} · ${qualityName(options.quality)}`
}
