import { Check, FolderOpen, Undo2 } from 'lucide-react'
import { formatBytes, formatCount } from '../utils/format'
import type { ConvertOptions, ConvertResult, ImageItem } from '../types'
import './SummaryCard.css'

interface Props {
  images: ImageItem[]
  results: Record<string, ConvertResult>
  options: ConvertOptions
  formatLabel: string
  onOpenFolder: () => void
  onUndo: () => void
}

/** The done-state replacement for the preview card. */
export function SummaryCard({
  images,
  results,
  options,
  formatLabel,
  onOpenFolder,
  onUndo,
}: Props) {
  const converted = Object.values(results).filter((result) => result.ok)
  const convertedIds = new Set(converted.map((result) => result.id))
  const succeeded = images.filter((image) => convertedIds.has(image.id))
  const failed = Object.values(results).filter((result) => !result.ok)

  // Only files that actually converted count toward "before": the ones that
  // failed kept their bytes, so including them would inflate the saving.
  const before = succeeded.reduce((sum, image) => sum + image.size, 0)
  const after = converted.reduce((sum, result) => sum + (result.outputSize ?? 0), 0)
  const saved = Math.max(0, before - after)
  const savedPercent = before > 0 ? Math.round((saved / before) * 100) : 0
  const barWidth = before > 0 ? Math.min(100, (after / before) * 100) : 0

  const qualityLabel = options.lossless
    ? 'lossless'
    : options.qualityMode === 'target'
      ? `${formatBytes(options.targetBytes)} target`
      : `quality ${options.quality}`

  let destination: string
  if (options.overwrite) {
    destination = 'Originals were replaced in place.'
  } else if (options.outputDir) {
    destination = `Saved to ${options.outputDir}.`
  } else if (options.filenameSuffix) {
    destination = `Saved beside each source file, "${options.filenameSuffix}" added to the name.`
  } else {
    destination = 'Saved beside each source file, under the same name.'
  }

  const hero = saved > 0 ? `Saved ${formatBytes(saved)}` : 'No space saved'

  return (
    <section className="summary-card" aria-label="Batch summary">
      <div className="summary-card__main">
        <div className="summary-card__headline">
          <span className="summary-card__badge" aria-hidden="true">
            <Check size={13} strokeWidth={3} />
          </span>
          {failed.length === 0 ? 'Batch complete' : `${converted.length} of ${images.length} converted`}
        </div>

        <h2 className="summary-card__hero display">{hero}</h2>

        <p className="summary-card__subtitle">
          {saved > 0 ? `${savedPercent}% smaller across ${formatCount(converted.length)}` : formatCount(converted.length)}
          {' · '}
          {formatLabel} at {qualityLabel}.
        </p>

        <div className="summary-card__actions">
          <button type="button" className="btn btn--primary" onClick={onOpenFolder}>
            <FolderOpen size={16} strokeWidth={1.75} aria-hidden="true" />
            Open output folder
          </button>
          <button type="button" className="btn btn--outline" onClick={onUndo}>
            <Undo2 size={16} strokeWidth={1.75} aria-hidden="true" />
            Undo
          </button>
        </div>
      </div>

      <div className="summary-card__sizes">
        <div className="summary-card__size-row">
          <div className="summary-card__size-label">
            <span>Before</span>
            <span className="numeric">{formatBytes(before)}</span>
          </div>
          <div className="summary-card__bar">
            <div className="summary-card__bar-fill is-before" style={{ width: '100%' }} />
          </div>
        </div>

        <div className="summary-card__size-row">
          <div className="summary-card__size-label">
            <span>After</span>
            <span className="numeric">{formatBytes(after)}</span>
          </div>
          <div className="summary-card__bar">
            <div
              className="summary-card__bar-fill is-after"
              style={{ width: `${barWidth}%` }}
            />
          </div>
        </div>

        <p className="summary-card__note">
          {destination}
          {failed.length > 0 ? ` ${failed.length} file${failed.length === 1 ? '' : 's'} could not be written.` : ''}
        </p>
      </div>
    </section>
  )
}
