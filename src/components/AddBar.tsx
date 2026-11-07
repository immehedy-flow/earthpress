import { ImagePlus } from 'lucide-react'
import './AddBar.css'

interface Props {
  onChooseImages: () => void
}

/**
 * The slim replacement for the 1.x hero drop zone. It stays put once files are
 * loaded, so the space below it can carry the preview.
 */
export function AddBar({ onChooseImages }: Props) {
  return (
    <div className="add-bar">
      <span className="add-bar__badge" aria-hidden="true">
        <ImagePlus size={18} strokeWidth={1.75} />
      </span>

      <div className="add-bar__text">
        <div className="add-bar__title">Drop images anywhere, or browse</div>
        <div className="add-bar__hint">
          PNG · JPG · WebP · AVIF · TIFF · BMP — files never leave this device
        </div>
      </div>

      <button type="button" className="add-bar__button" onClick={onChooseImages}>
        Add images
        <kbd className="add-bar__key">Ctrl O</kbd>
      </button>
    </div>
  )
}
