import { ImagePlus } from 'lucide-react'
import './DropOverlay.css'

interface Props {
  active: boolean
}

/** Full-window drop target shown while the OS drag is over the window. */
export function DropOverlay({ active }: Props) {
  if (!active) return null

  return (
    <div className="drop-overlay" aria-hidden="true">
      <div className="drop-overlay__card">
        <ImagePlus size={34} strokeWidth={1.5} />
        <span className="drop-overlay__title">Drop to add images</span>
        <span className="drop-overlay__hint">They stay on this device</span>
      </div>
    </div>
  )
}
