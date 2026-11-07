import { useEffect, useRef, useState } from 'react'
import { ArrowDown, ArrowUp, ArrowUpDown, Check, Trash2 } from 'lucide-react'
import { formatBytes, formatCount } from '../utils/format'
import type { SortField, SortOrder } from '../types'
import './BatchHeader.css'

interface Props {
  count: number
  totalSize: number
  sortField: SortField
  sortOrder: SortOrder
  onSortChange: (field: SortField) => void
  onClear: () => void
  disabled: boolean
}

const sortOptions: Array<{ value: SortField; label: string }> = [
  { value: 'added', label: 'Added' },
  { value: 'name', label: 'Name' },
  { value: 'size', label: 'Size' },
  { value: 'width', label: 'Width' },
  { value: 'height', label: 'Height' },
  { value: 'format', label: 'Format' },
]

/** "Batch" plus the count, the sort menu and Clear. */
export function BatchHeader({
  count,
  totalSize,
  sortField,
  sortOrder,
  onSortChange,
  onClear,
  disabled,
}: Props) {
  const [open, setOpen] = useState(false)
  const wrapper = useRef<HTMLDivElement>(null)

  // A menu that cannot be dismissed with a click elsewhere feels broken.
  useEffect(() => {
    if (!open) return

    const onPointerDown = (event: PointerEvent) => {
      if (!wrapper.current?.contains(event.target as Node)) setOpen(false)
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }

    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)

    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  const current = sortOptions.find((option) => option.value === sortField) ?? sortOptions[0]

  return (
    <header className="batch-header">
      <div className="batch-header__title">
        <span>Batch</span>
        <span className="batch-header__count numeric">
          {formatCount(count)} · {formatBytes(totalSize)}
        </span>
      </div>

      <div className="batch-header__actions">
        <div className="sort-menu" ref={wrapper}>
          <button
            type="button"
            className={`sort-menu__trigger ${open ? 'is-open' : ''}`}
            aria-haspopup="menu"
            aria-expanded={open}
            onClick={() => setOpen((value) => !value)}
          >
            <ArrowUpDown size={14} strokeWidth={1.75} aria-hidden="true" />
            Sort: {current.label}
          </button>

          {open ? (
            <div className="sort-menu__panel" role="menu">
              {sortOptions.map((option) => {
                const active = option.value === sortField
                return (
                  <button
                    key={option.value}
                    type="button"
                    role="menuitemradio"
                    aria-checked={active}
                    className={`sort-menu__item ${active ? 'is-active' : ''}`}
                    onClick={() => {
                      onSortChange(option.value)
                      setOpen(false)
                    }}
                  >
                    <span>{option.label}</span>
                    {active ? (
                      option.value === 'added' ? (
                        <Check size={14} strokeWidth={2.25} aria-hidden="true" />
                      ) : sortOrder === 'asc' ? (
                        <ArrowUp size={14} strokeWidth={2.25} aria-hidden="true" />
                      ) : (
                        <ArrowDown size={14} strokeWidth={2.25} aria-hidden="true" />
                      )
                    ) : null}
                  </button>
                )
              })}

              {sortField !== 'added' ? (
                <p className="sort-menu__note">
                  Pick the same field again to flip the order.
                </p>
              ) : null}
            </div>
          ) : null}
        </div>

        <button
          type="button"
          className="batch-header__clear"
          onClick={onClear}
          disabled={disabled}
        >
          <Trash2 size={14} strokeWidth={1.75} aria-hidden="true" />
          Clear
        </button>
      </div>
    </header>
  )
}
