import { useEffect } from 'react'

interface ShortcutMap {
  'ctrl+o': () => void;
  'ctrl+shift+o': () => void;
  'ctrl+enter': () => void;
  'escape': () => void;
  'delete': () => void;
  'ctrl+d': () => void;
  '?': () => void;
}

/** Typing in a field must never trigger a global shortcut. */
function isTextEntry(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT'
}

export function useKeyboardShortcuts(shortcuts: Partial<ShortcutMap>) {
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (isTextEntry(event.target)) return

      const ctrl = event.ctrlKey || event.metaKey
      const shift = event.shiftKey

      if (ctrl && !shift && event.key === 'o') {
        event.preventDefault()
        shortcuts['ctrl+o']?.()
      } else if (ctrl && shift && event.key === 'o') {
        event.preventDefault()
        shortcuts['ctrl+shift+o']?.()
      } else if (ctrl && !shift && event.key === 'Enter') {
        event.preventDefault()
        shortcuts['ctrl+enter']?.()
      } else if (event.key === 'Escape') {
        shortcuts['escape']?.()
      } else if (event.key === '?') {
        event.preventDefault()
        shortcuts['?']?.()
      } else if (event.key === 'Delete' || (ctrl && event.key === 'd')) {
        if (ctrl && event.key === 'd') {
          event.preventDefault()
        }
        shortcuts['delete']?.()
      }
    }

    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [shortcuts])
}