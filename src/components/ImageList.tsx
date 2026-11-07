import { useCallback, useRef } from 'react'
import { ImagePlus } from 'lucide-react'
import { ImageRow } from './ImageRow'
import type {
  ConversionProgress,
  ConvertResult,
  ImageEstimate,
  ImageItem,
  ImageTarget,
} from '../types'
import './ImageList.css'

/** Pixels of movement before a handle press counts as a drag. */
const DRAG_THRESHOLD = 4

interface Props {
  /** Already sorted - the order here is the order on screen. */
  images: ImageItem[]
  results: Record<string, ConvertResult>
  progress: Record<string, ConversionProgress>
  estimates: Record<string, ImageEstimate>
  targets: Record<string, ImageTarget>
  selectedId: string | null
  outputLabel: string
  isConverting: boolean
  reorderEnabled: boolean
  onSelect: (id: string) => void
  onRemove: (id: string) => void
  onRetry: (id: string) => void
  onReveal: (path: string) => void
  onReorder: (fromIndex: number, toIndex: number) => void
  dragActiveIndex: number | null
  onDragStart: (index: number) => void
  onDragEnd: () => void
}

/** Row index under the given viewport point, or null outside the list. */
function rowIndexAt(clientX: number, clientY: number): number | null {
  const element = document.elementFromPoint(clientX, clientY)
  const row = element?.closest('[data-row-index]')
  if (!row) return null

  const index = Number((row as HTMLElement).dataset.rowIndex)
  return Number.isInteger(index) ? index : null
}

export function ImageList({
  images,
  results,
  progress,
  estimates,
  targets,
  selectedId,
  outputLabel,
  isConverting,
  reorderEnabled,
  onSelect,
  onRemove,
  onRetry,
  onReveal,
  onReorder,
  dragActiveIndex,
  onDragStart,
  onDragEnd,
}: Props) {
  const listRef = useRef<HTMLUListElement | null>(null)
  const dragRef = useRef<{
    from: number
    pointerId: number
    startX: number
    startY: number
    active: boolean
  } | null>(null)

  /*
   * Reordering is driven by pointer events rather than HTML5 drag and drop.
   * Tauri replaces WebView2's drop handler so it can hand us real absolute file
   * paths from an OS drop, and that disables HTML5 drag and drop on Windows
   * entirely. Pointer events are unaffected by it, so the same gesture works.
   */
  const handlePointerDown = useCallback(
    (event: React.PointerEvent, index: number) => {
      if (event.button !== 0 || !reorderEnabled) return
      dragRef.current = {
        from: index,
        pointerId: event.pointerId,
        startX: event.clientX,
        startY: event.clientY,
        active: false,
      }
    },
    [reorderEnabled],
  )

  const handlePointerMove = useCallback(
    (event: React.PointerEvent<HTMLUListElement>) => {
      const drag = dragRef.current
      if (!drag || event.pointerId !== drag.pointerId) return

      if (!drag.active) {
        const travelled =
          Math.abs(event.clientX - drag.startX) + Math.abs(event.clientY - drag.startY)
        if (travelled < DRAG_THRESHOLD) return

        drag.active = true
        // Capture so the rest of the gesture is reported even between rows.
        listRef.current?.setPointerCapture(event.pointerId)
        onDragStart(drag.from)
      }

      event.preventDefault()

      const target = rowIndexAt(event.clientX, event.clientY)
      if (target === null || target === drag.from) return

      onReorder(drag.from, target)
      drag.from = target
      onDragStart(target)
    },
    [onDragStart, onReorder],
  )

  const endDrag = useCallback(
    (event: React.PointerEvent<HTMLUListElement>) => {
      const drag = dragRef.current
      if (!drag || event.pointerId !== drag.pointerId) return

      dragRef.current = null
      if (listRef.current?.hasPointerCapture(event.pointerId)) {
        listRef.current.releasePointerCapture(event.pointerId)
      }
      if (drag.active) onDragEnd()
    },
    [onDragEnd],
  )

  // Alt+Up/Down keeps reordering reachable without a pointer.
  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLUListElement>) => {
      if (!reorderEnabled || !event.altKey) return
      if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return

      const row = (event.target as HTMLElement).closest('[data-row-index]')
      if (!row) return

      const index = Number((row as HTMLElement).dataset.rowIndex)
      const target = event.key === 'ArrowUp' ? index - 1 : index + 1
      if (!Number.isInteger(index) || target < 0 || target >= images.length) return

      event.preventDefault()
      onReorder(index, target)
    },
    [images.length, onReorder, reorderEnabled],
  )

  if (images.length === 0) {
    return (
      <div className="image-list image-list--empty">
        <div className="empty-state">
          <span className="empty-state__icon" aria-hidden="true">
            <ImagePlus size={26} strokeWidth={1.5} />
          </span>
          <p className="empty-state__title">Your images will appear here</p>
          <p className="empty-state__hint">
            Add a few photos and EarthPress shows what each one will cost before you convert.
          </p>
        </div>
      </div>
    )
  }

  return (
    <ul
      className={`image-list ${dragActiveIndex !== null ? 'is-reordering' : ''}`}
      ref={listRef}
      onPointerMove={handlePointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={handleKeyDown}
    >
      {images.map((image, index) => (
        <ImageRow
          key={image.id}
          image={image}
          index={index}
          selected={image.id === selectedId}
          formatLabel={outputLabel}
          target={targets[image.id]}
          estimate={estimates[image.id]}
          result={results[image.id]}
          progress={progress[image.id]}
          isConverting={isConverting}
          reorderEnabled={reorderEnabled}
          isDragging={dragActiveIndex === index}
          onSelect={onSelect}
          onRemove={onRemove}
          onRetry={onRetry}
          onReveal={onReveal}
          dragHandleProps={{
            onPointerDown: (event: React.PointerEvent) => handlePointerDown(event, index),
          }}
        />
      ))}
    </ul>
  )
}
