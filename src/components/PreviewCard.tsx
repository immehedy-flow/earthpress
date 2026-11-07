import { useEffect, useRef, useState } from 'react'
import { ImageIcon, MoveHorizontal } from 'lucide-react'
import { FORMAT_INFO } from '../utils/formats'
import { formatBytes } from '../utils/format'
import type { ConvertOptions, ImageEstimate, ImageItem, ImageTarget, PreviewPayload } from '../types'
import './PreviewCard.css'

interface Props {
  image: ImageItem
  target?: ImageTarget
  estimate?: ImageEstimate
  options: ConvertOptions
}

/** Keeps a few rendered pairs around so switching rows is instant. */
const CACHE_LIMIT = 6

/** Settings settle before a preview is rendered; encoding is not free. */
const PREVIEW_DEBOUNCE = 220

/** Wide enough that a 1px difference in reported dimensions never matters. */
const SAME_SHAPE = 0.01

function dimensions(image: ImageItem, target?: ImageTarget) {
  const source = `${image.width} × ${image.height}`
  if (!target || (target.width === image.width && target.height === image.height)) {
    return source
  }

  return `${source} → ${target.width} × ${target.height}`
}

/**
 * The before/after card.
 *
 * Split compares the original with the encoded result at the divider; side by
 * side puts them next to each other, which is the only honest view once a crop
 * changes the shape.
 */
export function PreviewCard({ image, target, estimate, options }: Props) {
  const cache = useRef(new Map<string, PreviewPayload>())
  const lastImageId = useRef<string | null>(null)

  const [payload, setPayload] = useState<PreviewPayload | null>(null)
  const [loading, setLoading] = useState(true)
  const [failed, setFailed] = useState(false)
  const [mode, setMode] = useState<'split' | 'side'>('split')
  const [divider, setDivider] = useState(50)

  const formatLabel = FORMAT_INFO[options.format].label

  // Everything that changes the rendered result, and nothing that does not.
  const signature = JSON.stringify([
    image.id,
    options.format,
    options.quality,
    options.lossless,
    options.qualityMode,
    options.targetBytes,
    options.sizeMode,
    options.resizeKind,
    options.resizeValue,
    options.aspectPreset,
    options.customWidth,
    options.customHeight,
    options.fitMode,
    options.cropPosition,
    options.background,
    options.pngCompression,
    options.pngPalette,
  ])

  useEffect(() => {
    const cacheRef = cache.current

    // A different photo must never be shown under the old one's labels.
    if (lastImageId.current !== image.id) {
      lastImageId.current = image.id
      setPayload(null)
    }

    const cached = cacheRef.get(signature)
    if (cached) {
      setPayload(cached)
      setLoading(false)
      setFailed(false)
      return
    }

    let active = true
    setLoading(true)
    setFailed(false)

    const timer = window.setTimeout(() => {
      window.earthpress
        .previewPair(image.path, options)
        .then((next) => {
          cacheRef.set(signature, next)
          if (cacheRef.size > CACHE_LIMIT) {
            const oldest = cacheRef.keys().next().value
            if (oldest) cacheRef.delete(oldest)
          }
          if (active) setPayload(next)
        })
        .catch(() => {
          if (active) setFailed(true)
        })
        .finally(() => {
          if (active) setLoading(false)
        })
    }, PREVIEW_DEBOUNCE)

    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [image.id, image.path, options, signature])

  const sourceAspect = image.width / Math.max(1, image.height)
  const outputAspect = payload ? payload.outputWidth / Math.max(1, payload.outputHeight) : sourceAspect
  const canSplit = Math.abs(sourceAspect - outputAspect) < SAME_SHAPE
  const layout = canSplit ? mode : 'side'

  const afterLabel = estimate
    ? `${formatLabel} · ~${formatBytes(estimate.estimatedBytes)}`
    : `${formatLabel} · q${payload?.qualityUsed ?? options.quality}`
  const originalLabel = `Original · ${formatBytes(image.size)}`

  return (
    <section className="preview-card" aria-label="Preview">
      <header className="preview-card__header">
        <div className="preview-card__heading">
          <span className="preview-card__title">Preview</span>
          <span className="preview-card__name" title={image.name}>
            {image.name}
          </span>
        </div>

        <div className="preview-card__tools">
          <div className="preview-card__segmented" role="group" aria-label="Preview layout">
            <button
              type="button"
              className={`preview-card__segment ${layout === 'split' ? 'is-active' : ''}`}
              aria-pressed={layout === 'split'}
              disabled={!canSplit}
              title={
                canSplit
                  ? 'Compare at a divider'
                  : 'The output has a different shape, so compare side by side'
              }
              onClick={() => setMode('split')}
            >
              Split
            </button>
            <button
              type="button"
              className={`preview-card__segment ${layout === 'side' ? 'is-active' : ''}`}
              aria-pressed={layout === 'side'}
              onClick={() => setMode('side')}
            >
              Side by side
            </button>
          </div>
          <span className="preview-card__fit">Fit</span>
        </div>
      </header>

      <div className={`preview-card__stage ${layout === 'side' ? 'is-side-by-side' : ''}`}>
        {payload ? (
          layout === 'split' ? (
            <>
              <img
                className="preview-card__image"
                src={payload.beforeDataUrl}
                alt={`Original ${image.name}`}
                draggable={false}
              />
              <img
                className="preview-card__image preview-card__image--after"
                src={payload.afterDataUrl}
                alt=""
                aria-hidden="true"
                draggable={false}
                style={{ clipPath: `inset(0 0 0 ${divider}%)` }}
              />

              <span className="preview-card__label preview-card__label--left">{originalLabel}</span>
              <span className="preview-card__label preview-card__label--right">{afterLabel}</span>

              <div className="preview-card__divider" style={{ left: `${divider}%` }} aria-hidden="true">
                <span className="preview-card__handle">
                  <MoveHorizontal size={14} strokeWidth={2} />
                </span>
              </div>

              <input
                className="preview-card__scrubber"
                type="range"
                min={0}
                max={100}
                step={1}
                value={divider}
                aria-label="Compare the original and the converted image"
                onChange={(event) => setDivider(Number(event.target.value))}
              />
            </>
          ) : (
            <>
              <div className="preview-card__pane">
                <img
                  className="preview-card__image"
                  src={payload.beforeDataUrl}
                  alt={`Original ${image.name}`}
                  draggable={false}
                />
                <span className="preview-card__label preview-card__label--left">{originalLabel}</span>
              </div>
              <div className="preview-card__pane">
                <img
                  className="preview-card__image"
                  src={payload.afterDataUrl}
                  alt={`${formatLabel} result`}
                  draggable={false}
                />
                <span className="preview-card__label preview-card__label--left">{afterLabel}</span>
              </div>
            </>
          )
        ) : (
          <div className="preview-card__placeholder">
            <ImageIcon size={22} strokeWidth={1.5} aria-hidden="true" />
            <span>{failed ? 'Preview unavailable for this file' : 'Rendering preview…'}</span>
          </div>
        )}

        {loading && payload ? <span className="preview-card__busy">Rendering…</span> : null}
      </div>

      <footer className="preview-card__footer">
        <span className="numeric">{dimensions(image, target)}</span>
        <span className="preview-card__note">
          {formatLabel} · {options.lossless ? 'lossless' : `q${payload?.qualityUsed ?? options.quality}`}
          {estimate ? ` · ~${formatBytes(estimate.estimatedBytes)}` : ''}
        </span>
      </footer>
    </section>
  )
}
