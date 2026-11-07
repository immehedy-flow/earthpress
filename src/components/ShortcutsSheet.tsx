import { useEffect, useRef } from 'react'
import { X } from 'lucide-react'
import './ShortcutsSheet.css'

interface Props {
  open: boolean
  onClose: () => void
}

/** Kept in step with `useKeyboardShortcuts`. */
const shortcuts: Array<{ label: string; keys: string[] }> = [
  { label: 'Add images', keys: ['Ctrl', 'O'] },
  { label: 'Choose output folder', keys: ['Ctrl', 'Shift', 'O'] },
  { label: 'Convert the batch', keys: ['Ctrl', '↵'] },
  { label: 'Move a selected image', keys: ['Alt', '↑', '↓'] },
  { label: 'Cancel a running batch', keys: ['Esc'] },
  { label: 'Remove all images', keys: ['Delete'] },
  { label: 'Show this sheet', keys: ['?'] },
]

/** The sheet behind the title bar's `?` button. */
export function ShortcutsSheet({ open, onClose }: Props) {
  const closeRef = useRef<HTMLButtonElement>(null)

  // Move focus into the sheet so Escape reaches the shortcut hook.
  useEffect(() => {
    if (open) closeRef.current?.focus()
  }, [open])

  if (!open) return null

  return (
    <div className="sheet-backdrop" role="presentation" onClick={onClose}>
      <div
        className="sheet"
        role="dialog"
        aria-modal="true"
        aria-label="Keyboard shortcuts"
        onClick={(event) => event.stopPropagation()}
      >
        <header className="sheet__header">
          <h2 className="display">Keyboard shortcuts</h2>
          <button
            ref={closeRef}
            type="button"
            className="sheet__close"
            aria-label="Close"
            onClick={onClose}
          >
            <X size={16} strokeWidth={2} aria-hidden="true" />
          </button>
        </header>

        <dl className="sheet__list">
          {shortcuts.map(({ label, keys }) => (
            <div className="sheet__row" key={label}>
              <dt>{label}</dt>
              <dd>
                {keys.map((key) => (
                  <kbd className="keycap" key={key}>
                    {key}
                  </kbd>
                ))}
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </div>
  )
}
