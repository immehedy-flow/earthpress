import { Check, FolderOpen, GripVertical, RotateCcw, Trash2 } from 'lucide-react'
import { sourceLabel } from '../utils/formats'
import { formatBytes, formatPercent } from '../utils/format'
import type { ConversionProgress, ConvertResult, ImageEstimate, ImageItem, ImageTarget } from '../types'
import './ImageRow.css'

interface Props {
  image: ImageItem
  index: number
  selected: boolean
  formatLabel: string
  target?: ImageTarget
  estimate?: ImageEstimate
  result?: ConvertResult
  progress?: ConversionProgress
  isConverting: boolean
  reorderEnabled: boolean
  isDragging: boolean
  onSelect: (id: string) => void
  onRemove: (id: string) => void
  onRetry: (id: string) => void
  onReveal: (path: string) => void
  /** Spread onto the grip; the pointer logic lives in `ImageList`. */
  dragHandleProps: Record<string, unknown>
}

function dimensions(image: ImageItem, target?: ImageTarget) {
  const source = `${image.width} × ${image.height}`
  if (!target || (target.width === image.width && target.height === image.height)) {
    return source
  }

  return `${source} → ${target.width} × ${target.height}`
}

/**
 * One row of the batch list.
 *
 * Ready rows carry the live estimate (or the source size before the estimate
 * lands), working rows carry progress, done rows carry the real output size and
 * failed rows say what went wrong.
 */
export function ImageRow({
  image,
  index,
  selected,
  formatLabel,
  target,
  estimate,
  result,
  progress,
  isConverting,
  reorderEnabled,
  isDragging,
  onSelect,
  onRemove,
  onRetry,
  onReveal,
  dragHandleProps,
}: Props) {
  const isDone = result?.ok === true
  const isFailed = result != null && !result.ok
  const percent = progress == null ? null : Math.round(progress.progress)

  const outputBytes = result?.outputSize ?? 0
  const comparison = isDone
    ? outputBytes
    : estimate?.estimatedBytes
  const ratio = comparison == null || image.size <= 0 ? null : Math.min(100, (comparison / image.size) * 100)

  let line: string | null = null
  if (isDone) {
    line = formatBytes(outputBytes)
  } else if (isFailed) {
    line = result?.error ?? 'Conversion failed'
  } else if (isConverting) {
    line = percent == null ? 'Queued' : `Converting · ${percent}%`
  } else if (estimate) {
    line = `${formatLabel} · ~${formatBytes(estimate.estimatedBytes)}`
  }

  const pill = isDone
    ? formatPercent(outputBytes, image.size)
    : isFailed
      ? 'Failed'
      : isConverting
        ? percent == null
          ? null
          : `${percent}%`
        : estimate
          ? formatPercent(estimate.estimatedBytes, image.size)
          : null

  const barWidth = isConverting ? (percent ?? 0) : isFailed ? 0 : (ratio ?? 0)

  const classes = [
    'image-row',
    selected ? 'is-selected' : '',
    isDragging ? 'is-dragging' : '',
    isDone ? 'is-done' : '',
    isFailed ? 'is-failed' : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <li className={classes} data-row-index={index}>
      <span
        className={`image-row__grip ${reorderEnabled ? '' : 'is-disabled'}`}
        title={reorderEnabled ? 'Drag to reorder' : 'Set Sort to “Added” to reorder'}
        aria-hidden="true"
        {...dragHandleProps}
      >
        <GripVertical size={14} strokeWidth={2} />
      </span>

      <button
        type="button"
        className="image-row__body"
        aria-current={selected ? 'true' : undefined}
        onClick={() => onSelect(image.id)}
      >
        {image.preview ? (
          <img className="image-row__thumb" src={image.preview} alt="" draggable={false} />
        ) : (
          <span className="image-row__thumb image-row__thumb--empty" aria-hidden="true" />
        )}

        <span className="image-row__text">
          <span className="image-row__name" title={image.name}>
            {image.name}
          </span>
          <span className="image-row__meta">
            {dimensions(image, target)} · {sourceLabel(image.format)} · {formatBytes(image.size)}
          </span>
        </span>
      </button>

      {line ? (
        <div className="image-row__estimate">
          <div className={`image-row__line ${isFailed ? 'is-failed' : ''}`}>
            {isDone ? (
              <span className="image-row__check" aria-hidden="true">
                <Check size={14} strokeWidth={2.5} />
              </span>
            ) : null}
            <span className="image-row__line-text" title={line}>
              {line}
            </span>
          </div>
          <div className="image-row__track">
            <div
              className={`image-row__fill ${isFailed ? 'is-failed' : ''}`}
              style={{ width: `${barWidth}%` }}
            />
          </div>
        </div>
      ) : (
        <div className="image-row__estimate" />
      )}

      {pill ? (
        <span className={`image-row__pill ${isFailed ? 'is-failed' : ''}`}>{pill}</span>
      ) : (
        <span className="image-row__pill is-empty" aria-hidden="true" />
      )}

      {isDone ? (
        <button
          type="button"
          className="image-row__action"
          aria-label={`Show ${image.name} in Explorer`}
          title="Show in Explorer"
          onClick={() => onReveal(result?.outputPath ?? image.path)}
        >
          <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      ) : isFailed ? (
        <button
          type="button"
          className="image-row__action"
          aria-label={`Retry ${image.name}`}
          title="Retry this image"
          onClick={() => onRetry(image.id)}
        >
          <RotateCcw size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      ) : (
        <button
          type="button"
          className="image-row__action"
          aria-label={`Remove ${image.name}`}
          title="Remove"
          onClick={() => onRemove(image.id)}
        >
          <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      )}
    </li>
  )
}
