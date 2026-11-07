export function formatBytes(bytes = 0) {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let size = bytes / 1024
  let unit = units[0]
  for (let index = 1; index < units.length && size >= 1024; index += 1) {
    size /= 1024
    unit = units[index]
  }
  return `${size.toFixed(size >= 10 ? 1 : 2)} ${unit}`
}

export function formatName(format: string) {
  return format === 'jpg' ? 'JPG' : format.toUpperCase()
}

export function formatRatio(ratio: string) {
  return ratio.replace(':', ' × ')
}

/**
 * "−86%" when the output is smaller, "+12%" when it grew, "0%" when unchanged.
 * Uses the typographic minus, matching the mockups.
 */
export function formatPercent(after: number, before: number) {
  if (before <= 0) return '0%'
  const percent = Math.round((1 - after / before) * 100)
  if (percent > 0) return `−${percent}%`
  if (percent < 0) return `+${Math.abs(percent)}%`
  return '0%'
}

/** "3 files" / "1 file" */
export function formatCount(count: number) {
  return `${count} file${count === 1 ? '' : 's'}`
}