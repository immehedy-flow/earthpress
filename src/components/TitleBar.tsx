import { useEffect, useState } from 'react'
import { CircleHelp, Leaf, Lock, Minus, Monitor, Moon, Square, Sun, X } from 'lucide-react'
import { windowControls } from '../bridge'
import { formatBytes, formatCount } from '../utils/format'
import type { ConvertResult, ImageEstimate, ImageItem, ThemePreference } from '../types'
import './TitleBar.css'

interface Props {
  images: ImageItem[]
  results: Record<string, ConvertResult>
  /** Absent until the estimate pass has answered once. */
  estimates?: Record<string, ImageEstimate>
  preference: ThemePreference
  onSelectTheme: (preference: ThemePreference) => void
  onShowShortcuts: () => void
}

const themeOptions: Array<{ value: ThemePreference; label: string; Icon: typeof Sun }> = [
  { value: 'light', label: 'Light theme', Icon: Sun },
  { value: 'dark', label: 'Dark theme', Icon: Moon },
  { value: 'system', label: 'Follow system theme', Icon: Monitor },
]

/** The Windows "restore" glyph, which the mockup does not draw. */
function RestoreGlyph() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="9" y="9" width="12" height="12" rx="2" />
      <path d="M15 5H6a2 2 0 0 0-2 2v9" />
    </svg>
  )
}

/**
 * The app's own title bar.
 *
 * The window is undecorated (`decorations: false`), so this replaces the native
 * frame: the drag region, the always-visible savings chip, the theme switch and
 * the window buttons all live here.
 */
export function TitleBar({
  images,
  results,
  estimates = {},
  preference,
  onSelectTheme,
  onShowShortcuts,
}: Props) {
  const [isMaximized, setIsMaximized] = useState(false)

  // Maximizing and restoring both resize, so that is enough to stay in sync.
  useEffect(() => {
    let active = true
    const sync = () => {
      void windowControls.isMaximized().then((value) => {
        if (active) setIsMaximized(value)
      })
    }

    sync()
    const unsubscribe = windowControls.onResized(sync)

    return () => {
      active = false
      unsubscribe()
    }
  }, [])

  const converted = Object.values(results).filter((result) => result.ok)
  const convertedIds = new Set(converted.map((result) => result.id))
  const succeeded = images.filter((image) => convertedIds.has(image.id))
  const isDone = converted.length > 0
  const failedCount = isDone ? images.length - succeeded.length : 0

  // Once the batch has run, compare only the files that actually converted -
  // a failed file kept its bytes, so counting it as "saved" would be a lie.
  const sourceTotal = isDone
    ? succeeded.reduce((sum, image) => sum + image.size, 0)
    : images.reduce((sum, image) => sum + image.size, 0)

  const estimatedTotal = images.reduce(
    (sum, image) => sum + (estimates[image.id]?.estimatedBytes ?? 0),
    0,
  )
  const hasEstimates = images.length > 0 && images.every((image) => estimates[image.id])

  const afterTotal = isDone
    ? converted.reduce((sum, result) => sum + (result.outputSize ?? 0), 0)
    : estimatedTotal

  const savedPercent = sourceTotal > 0 ? Math.round((1 - afterTotal / sourceTotal) * 100) : 0

  let chipText: string | null = null
  if (images.length > 0) {
    if (isDone) {
      const counted = failedCount > 0 ? `${succeeded.length} of ${formatCount(images.length)}` : formatCount(succeeded.length)
      chipText = `${counted} · ${formatBytes(sourceTotal)} → ${formatBytes(afterTotal)} · saved ${savedPercent}%`
    } else if (hasEstimates) {
      chipText = `${formatCount(images.length)} · ${formatBytes(sourceTotal)} → ~${formatBytes(afterTotal)} · saves ~${savedPercent}%`
    } else {
      chipText = `${formatCount(images.length)} · ${formatBytes(sourceTotal)}`
    }
  }

  return (
    <header className="titlebar" data-tauri-drag-region="deep">
      <div className="titlebar__side titlebar__side--left">
        <span className="titlebar__mark" aria-hidden="true">
          <Leaf size={15} strokeWidth={2} />
        </span>
        <span className="titlebar__brand display">EarthPress</span>
        <span className="titlebar__offline">
          <Lock size={11} strokeWidth={2} aria-hidden="true" />
          100% offline
        </span>
      </div>

      <div className="titlebar__center">
        {chipText ? (
          <span className="titlebar__chip" title={chipText}>
            <Leaf size={14} strokeWidth={2} aria-hidden="true" />
            <span className="titlebar__chip-text numeric">{chipText}</span>
          </span>
        ) : null}
      </div>

      <div className="titlebar__side titlebar__side--right">
        <div className="theme-switch" role="group" aria-label="Theme">
          {themeOptions.map(({ value, label, Icon }) => (
            <button
              key={value}
              type="button"
              className={`theme-switch__button ${preference === value ? 'is-active' : ''}`}
              aria-label={label}
              aria-pressed={preference === value}
              title={label}
              onClick={() => onSelectTheme(value)}
            >
              <Icon size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          ))}
        </div>

        <button
          type="button"
          className="titlebar__help"
          aria-label="Keyboard shortcuts"
          title="Keyboard shortcuts"
          onClick={onShowShortcuts}
        >
          <CircleHelp size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>

        <div className="window-controls">
          <button
            type="button"
            className="window-controls__button"
            aria-label="Minimize"
            onClick={() => void windowControls.minimize()}
          >
            <Minus size={12} strokeWidth={2} aria-hidden="true" />
          </button>
          <button
            type="button"
            className="window-controls__button"
            aria-label={isMaximized ? 'Restore' : 'Maximize'}
            onClick={() => void windowControls.toggleMaximize()}
          >
            {isMaximized ? <RestoreGlyph /> : <Square size={12} strokeWidth={2} aria-hidden="true" />}
          </button>
          <button
            type="button"
            className="window-controls__button window-controls__button--close"
            aria-label="Close"
            onClick={() => void windowControls.close()}
          >
            <X size={12} strokeWidth={2} aria-hidden="true" />
          </button>
        </div>
      </div>
    </header>
  )
}
