import type { ConvertOptions, Preset } from '../types'

const STORAGE_KEY = 'earthpress-presets'

export function loadPresets(): Preset[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    return raw ? JSON.parse(raw) : []
  } catch {
    return []
  }
}

export function savePreset(name: string, options: ConvertOptions) {
  const presets = loadPresets()
  const filtered: Partial<ConvertOptions> = {
    format: options.format,
    quality: options.quality,
    lossless: options.lossless,
    sizeMode: options.sizeMode,
    resizeKind: options.resizeKind,
    resizeValue: options.resizeValue,
    aspectPreset: options.aspectPreset,
    customWidth: options.customWidth,
    customHeight: options.customHeight,
    fitMode: options.fitMode,
    cropPosition: options.cropPosition,
    background: options.background,
    pngCompression: options.pngCompression,
    pngPalette: options.pngPalette,
  }
  presets.push({ name, options: filtered })
  localStorage.setItem(STORAGE_KEY, JSON.stringify(presets))
  return presets
}

export function deletePreset(name: string) {
  const presets = loadPresets().filter((p) => p.name !== name)
  localStorage.setItem(STORAGE_KEY, JSON.stringify(presets))
  return presets
}

export function applyPreset(preset: Preset, current: ConvertOptions): ConvertOptions {
  return { ...current, ...preset.options }
}