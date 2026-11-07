import { useEffect, useRef, useState } from 'react'
import {
  ArrowRight,
  ChevronDown,
  FolderOpen,
  Loader2,
  Plus,
  Square,
  X,
} from 'lucide-react'
import { deletePreset, loadPresets, savePreset } from '../utils/presets'
import {
  FORMATS,
  FORMAT_INFO,
  QUALITY_LEVELS,
  describeOptions,
  describeSize,
} from '../utils/formats'
import { formatBytes, formatCount } from '../utils/format'
import type {
  AspectPreset,
  ConvertOptions,
  CropPosition,
  FitMode,
  ImageFormat,
  Preset,
  ResizeKind,
  SizeMode,
} from '../types'
import './SettingsPanel.css'

interface Props {
  options: ConvertOptions
  onChange: <K extends keyof ConvertOptions>(key: K, value: ConvertOptions[K]) => void
  onApplyPreset: (preset: Preset) => void
  onChooseOutputDir: () => void
  onConvert: () => void
  onCancel: () => void
  isConverting: boolean
  count: number
  totalSize: number
  /** The batch has run at least once, so the button offers a re-run. */
  hasResults: boolean
  /** Measured output size of the selected image, when there is one. */
  sizeReadout: string | null
  /** How many files cannot fit the byte target even at the lowest quality. */
  targetMisses: number
}

const SIZE_MODES: Array<{ value: SizeMode; label: string }> = [
  { value: 'original', label: 'Original' },
  { value: 'resize', label: 'Resize' },
  { value: 'aspect', label: 'Ratio' },
  { value: 'custom', label: 'Custom' },
]

const RESIZE_KINDS: Array<{ value: ResizeKind; label: string }> = [
  { value: 'max', label: 'Max side' },
  { value: 'width', label: 'Width' },
  { value: 'height', label: 'Height' },
  { value: 'percent', label: 'Percent' },
]

const ASPECT_PRESETS: AspectPreset[] = ['1:1', '4:5', '5:4', '4:3', '3:2', '16:9', '9:16']

/** 20 MB is well past any sane web target and keeps the field honest. */
const MAX_TARGET_KB = 20 * 1024

const FIT_MODES: Array<{ value: FitMode; label: string }> = [
  { value: 'cover', label: 'Cover — crop to fill' },
  { value: 'contain', label: 'Contain — fit inside' },
  { value: 'fill', label: 'Fill — stretch' },
  { value: 'inside', label: 'Inside — shrink only' },
  { value: 'outside', label: 'Outside — fill, may enlarge' },
]

const CROP_POSITIONS: Array<{ value: CropPosition; label: string }> = [
  { value: 'center', label: 'Centre' },
  { value: 'top', label: 'Top' },
  { value: 'bottom', label: 'Bottom' },
  { value: 'left', label: 'Left' },
  { value: 'right', label: 'Right' },
  { value: 'attention', label: 'Smart (attention)' },
  { value: 'entropy', label: 'Smart (entropy)' },
]

/** Numeric input that tolerates an empty box while typing. */
function NumberField({
  value,
  onChange,
  label,
  min = 1,
  max = 100000,
}: {
  value: number
  onChange: (value: number) => void
  label: string
  min?: number
  max?: number
}) {
  const [draft, setDraft] = useState(String(value))
  const [lastValue, setLastValue] = useState(value)

  // When the parent corrects the value, follow it. Adjusting during render
  // keeps the caret in place while the user is still typing.
  if (value !== lastValue) {
    setLastValue(value)
    setDraft(String(value))
  }

  const commit = (raw: string) => {
    setDraft(raw)
    const parsed = Number(raw)
    if (Number.isFinite(parsed) && parsed >= min && parsed <= max) onChange(parsed)
  }

  return (
    <input
      className="control control--number"
      inputMode="numeric"
      aria-label={label}
      value={draft}
      onChange={(event) => commit(event.target.value)}
      onBlur={() => {
        const parsed = Number(draft)
        if (!Number.isFinite(parsed) || parsed < min || parsed > max) setDraft(String(value))
      }}
    />
  )
}

// ---------------------------------------------------------------------------
// Preset bar
// ---------------------------------------------------------------------------

function PresetBar({
  options,
  onApplyPreset,
}: {
  options: ConvertOptions
  onApplyPreset: (preset: Preset) => void
}) {
  const [open, setOpen] = useState(false)
  const [presets, setPresets] = useState<Preset[]>(() => loadPresets())
  const [naming, setNaming] = useState(false)
  const [name, setName] = useState('')
  const wrapper = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return

    const onPointerDown = (event: PointerEvent) => {
      if (!wrapper.current?.contains(event.target as Node)) {
        setOpen(false)
        setNaming(false)
      }
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpen(false)
        setNaming(false)
      }
    }

    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)

    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  const commitName = () => {
    const trimmed = name.trim()
    if (!trimmed) return
    setPresets(savePreset(trimmed, options))
    setName('')
    setNaming(false)
  }

  return (
    <div className="preset-bar">
      <div className="preset-menu" ref={wrapper}>
        <button
          type="button"
          className="preset-menu__trigger"
          aria-haspopup="menu"
          aria-expanded={open}
          onClick={() => setOpen((value) => !value)}
        >
          <span className="preset-menu__icon" aria-hidden="true">
            <Square size={15} strokeWidth={1.75} />
          </span>
          <span className="preset-menu__label">Preset</span>
          <span className="preset-menu__value">{describeOptions(options)}</span>
          <ChevronDown size={13} strokeWidth={2} aria-hidden="true" />
        </button>

        {open ? (
          <div className="preset-menu__panel" role="menu">
            <p className="preset-menu__group">Quality levels</p>
            {QUALITY_LEVELS.map((level) => (
              <button
                key={level.value}
                type="button"
                role="menuitemradio"
                aria-checked={options.quality === level.value && !options.lossless}
                className={`preset-menu__item ${
                  options.quality === level.value && !options.lossless ? 'is-active' : ''
                }`}
                onClick={() => {
                  onApplyPreset({ name: level.label, options: { quality: level.value, lossless: false } })
                  setOpen(false)
                }}
              >
                <span>{level.label}</span>
                <span className="preset-menu__meta numeric">{level.value}</span>
              </button>
            ))}

            {presets.length > 0 ? (
              <>
                <p className="preset-menu__group">Saved presets</p>
                {presets.map((preset) => (
                  <div key={preset.name} className="preset-menu__saved">
                    <button
                      type="button"
                      role="menuitem"
                      className="preset-menu__item preset-menu__item--saved"
                      onClick={() => {
                        onApplyPreset(preset)
                        setOpen(false)
                      }}
                    >
                      {preset.name}
                    </button>
                    <button
                      type="button"
                      className="preset-menu__delete"
                      aria-label={`Delete preset ${preset.name}`}
                      onClick={() => setPresets(deletePreset(preset.name))}
                    >
                      <X size={13} strokeWidth={2} />
                    </button>
                  </div>
                ))}
              </>
            ) : null}

            <div className="preset-menu__footer">
              {naming ? (
                <div className="preset-menu__form">
                  <input
                    className="control"
                    autoFocus
                    placeholder="Preset name"
                    value={name}
                    aria-label="Preset name"
                    onChange={(event) => setName(event.target.value)}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter') commitName()
                    }}
                  />
                  <button type="button" className="btn btn--sm btn--primary" onClick={commitName}>
                    Save
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  className="preset-menu__save"
                  onClick={() => setNaming(true)}
                >
                  <Plus size={14} strokeWidth={2} aria-hidden="true" />
                  Save current settings…
                </button>
              )}
            </div>
          </div>
        ) : null}
      </div>

      <button
        type="button"
        className="preset-bar__save"
        aria-label="Save current settings as a preset"
        title="Save current settings as a preset"
        onClick={() => {
          setOpen(true)
          setNaming(true)
        }}
      >
        <Plus size={16} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Format, quality, size, output
// ---------------------------------------------------------------------------

function FormatPicker({
  format,
  onChange,
}: {
  format: ImageFormat
  onChange: (format: ImageFormat) => void
}) {
  return (
    <div>
      <div className="segmented segmented--format" role="group" aria-label="Output format">
        {FORMATS.map((entry) => (
          <button
            key={entry.value}
            type="button"
            className={`segmented__item ${format === entry.value ? 'is-active' : ''}`}
            aria-pressed={format === entry.value}
            onClick={() => onChange(entry.value)}
          >
            {entry.label}
          </button>
        ))}
      </div>
      <p className="settings-hint">{FORMAT_INFO[format].hint}</p>
    </div>
  )
}

function QualityCard({
  options,
  onChange,
  targetMisses,
}: {
  options: ConvertOptions
  onChange: Props['onChange']
  targetMisses: number
}) {
  const info = FORMAT_INFO[options.format]
  const losslessDisabled = !info.lossless

  if (options.format === 'png') {
    return (
      <section className="settings-card">
        <div className="settings-card__head">
          <span className="settings-card__title">Compression</span>
          <span className="settings-card__value numeric">{options.pngCompression}</span>
        </div>

        <input
          className="slider"
          type="range"
          min={0}
          max={9}
          step={1}
          value={options.pngCompression}
          aria-label="PNG compression"
          onChange={(event) => onChange('pngCompression', Number(event.target.value))}
        />
        <p className="settings-hint settings-hint--tight">
          0 is fastest, 9 packs tightest. Either way the pixels are lossless.
        </p>

        <label className="check-row">
          <input
            type="checkbox"
            checked={options.pngPalette}
            onChange={(event) => onChange('pngPalette', event.target.checked)}
          />
          <span>
            Reduce to a 256-colour palette
            <span className="check-row__hint">Smaller files, visible banding on gradients.</span>
          </span>
        </label>
      </section>
    )
  }

  const canTarget = info.quality && !options.lossless
  const isTarget = options.qualityMode === 'target' && canTarget
  const targetKilobytes = Math.max(1, Math.round(options.targetBytes / 1024))

  return (
    <section className="settings-card">
      <div className="settings-card__head">
        <span className="settings-card__title">Quality</span>
        <div className="mini-segmented" role="group" aria-label="Quality mode">
          <button
            type="button"
            className={`mini-segmented__item ${isTarget ? '' : 'is-active'}`}
            aria-pressed={!isTarget}
            onClick={() => onChange('qualityMode', 'level')}
          >
            Level
          </button>
          <button
            type="button"
            className={`mini-segmented__item ${isTarget ? 'is-active' : ''}`}
            aria-pressed={isTarget}
            disabled={!canTarget}
            title={
              canTarget
                ? 'Aim for a byte size instead of a fixed quality'
                : 'Turn off lossless to target a file size'
            }
            onClick={() => onChange('qualityMode', 'target')}
          >
            Target size
          </button>
        </div>
      </div>

      {isTarget ? (
        <>
          <div className="size-row">
            <NumberField
              label="Target size in kilobytes"
              value={targetKilobytes}
              min={1}
              max={MAX_TARGET_KB}
              onChange={(kilobytes) => onChange('targetBytes', kilobytes * 1024)}
            />
            <span className="size-row__unit">KB</span>
          </div>
          <p className="settings-hint settings-hint--tight">
            Each file gets the highest quality that still fits. Two files of the same size can
            need different qualities.
          </p>
          {targetMisses > 0 ? (
            <p className="settings-warning">
              {formatCount(targetMisses)} cannot reach {targetKilobytes} KB even at the lowest
              quality.
            </p>
          ) : null}
        </>
      ) : (
        <>
          <div className={`quality-slider ${options.lossless ? 'is-muted' : ''}`}>
            <input
              className="slider"
              type="range"
              min={1}
              max={100}
              step={1}
              value={options.quality}
              aria-label="Quality"
              disabled={options.lossless}
              onChange={(event) => onChange('quality', Number(event.target.value))}
            />
            <span className="quality-slider__value display">{options.quality}</span>
          </div>

          <div className="quality-chips">
            {QUALITY_LEVELS.map((level) => {
              const active = !options.lossless && options.quality === level.value
              return (
                <button
                  key={level.value}
                  type="button"
                  className={`quality-chip ${active ? 'is-active' : ''}`}
                  aria-pressed={active}
                  onClick={() => {
                    onChange('quality', level.value)
                    if (options.lossless) onChange('lossless', false)
                  }}
                >
                  <span className="quality-chip__label">{level.label}</span>
                  <span className="quality-chip__value">{level.value}</span>
                </button>
              )
            })}
          </div>
        </>
      )}

      <label className={`check-row ${losslessDisabled ? 'is-disabled' : ''}`}>
        <input
          type="checkbox"
          checked={options.lossless && !losslessDisabled}
          disabled={losslessDisabled}
          onChange={(event) => {
            const checked = event.target.checked
            onChange('lossless', checked)
            // Lossless ignores the quality dial, so a byte target cannot hold.
            if (checked) onChange('qualityMode', 'level')
          }}
        />
        <span>
          Lossless
          <span className="check-row__hint">
            {losslessDisabled
              ? `Not available for ${FORMAT_INFO[options.format].label}.`
              : 'Keeps every pixel; the slider no longer applies.'}
          </span>
        </span>
      </label>
    </section>
  )
}

function SizeCard({
  options,
  onChange,
  sizeReadout,
}: {
  options: ConvertOptions
  onChange: Props['onChange']
  sizeReadout: string | null
}) {
  // Ratio and Custom crop the image, so they need the fit controls. Original
  // and Resize keep the whole frame and never need them.
  const needsFit = options.sizeMode === 'aspect' || options.sizeMode === 'custom'

  return (
    <section className="settings-card">
      <div className="segmented segmented--size" role="group" aria-label="Output size">
        {SIZE_MODES.map((mode) => (
          <button
            key={mode.value}
            type="button"
            className={`segmented__item ${options.sizeMode === mode.value ? 'is-active' : ''}`}
            aria-pressed={options.sizeMode === mode.value}
            onClick={() => onChange('sizeMode', mode.value)}
          >
            {mode.label}
          </button>
        ))}
      </div>

      {options.sizeMode === 'resize' ? (
        <div className="size-row">
          <div className="size-row__kind">
            <select
              className="control control--select"
              aria-label="Resize by"
              value={options.resizeKind}
              onChange={(event) => onChange('resizeKind', event.target.value as ResizeKind)}
            >
              {RESIZE_KINDS.map((kind) => (
                <option key={kind.value} value={kind.value}>
                  {kind.label}
                </option>
              ))}
            </select>
          </div>
          <NumberField
            label="Resize amount"
            value={options.resizeValue}
            min={1}
            max={options.resizeKind === 'percent' ? 400 : 100000}
            onChange={(value) => onChange('resizeValue', value)}
          />
          <span className="size-row__unit">{options.resizeKind === 'percent' ? '%' : 'px'}</span>
        </div>
      ) : null}

      {options.sizeMode === 'aspect' ? (
        <div className="chip-row">
          {ASPECT_PRESETS.map((preset) => (
            <button
              key={preset}
              type="button"
              className={`ratio-chip ${options.aspectPreset === preset ? 'is-active' : ''}`}
              aria-pressed={options.aspectPreset === preset}
              onClick={() => onChange('aspectPreset', preset)}
            >
              {preset}
            </button>
          ))}
        </div>
      ) : null}

      {options.sizeMode === 'custom' ? (
        <div className="size-row">
          <NumberField
            label="Width in pixels"
            value={options.customWidth}
            onChange={(value) => onChange('customWidth', value)}
          />
          <span className="size-row__unit">×</span>
          <NumberField
            label="Height in pixels"
            value={options.customHeight}
            onChange={(value) => onChange('customHeight', value)}
          />
          <span className="size-row__unit">px</span>
        </div>
      ) : null}

      {needsFit ? (
        <>
          <div className="field">
            <label className="field__label" htmlFor="fit-mode">
              Fit
            </label>
            <select
              id="fit-mode"
              className="control control--select"
              value={options.fitMode}
              onChange={(event) => onChange('fitMode', event.target.value as FitMode)}
            >
              {FIT_MODES.map((mode) => (
                <option key={mode.value} value={mode.value}>
                  {mode.label}
                </option>
              ))}
            </select>
          </div>

          {options.fitMode === 'cover' ? (
            <div className="field">
              <label className="field__label" htmlFor="crop-position">
                Crop focus
              </label>
              <select
                id="crop-position"
                className="control control--select"
                value={options.cropPosition}
                onChange={(event) =>
                  onChange('cropPosition', event.target.value as CropPosition)
                }
              >
                {CROP_POSITIONS.map((position) => (
                  <option key={position.value} value={position.value}>
                    {position.label}
                  </option>
                ))}
              </select>
            </div>
          ) : null}

          <div className="field">
            <label className="field__label" htmlFor="background">
              Background
            </label>
            <div className="field__row">
              <input
                id="background"
                className="control"
                value={options.background}
                spellCheck={false}
                onChange={(event) => onChange('background', event.target.value)}
              />
              <input
                className="color-well"
                type="color"
                aria-label="Background colour"
                value={/^#[0-9a-f]{6}$/i.test(options.background) ? options.background : '#ffffff'}
                onChange={(event) => onChange('background', event.target.value)}
              />
            </div>
          </div>
        </>
      ) : null}

      <div className="size-readout">
        <span className="size-readout__arrow" aria-hidden="true">
          <ArrowRight size={14} strokeWidth={2} />
        </span>
        <span className="size-readout__text">{sizeReadout ?? describeSize(options)}</span>
      </div>
    </section>
  )
}

function OutputCard({
  options,
  onChange,
  onChooseOutputDir,
}: {
  options: ConvertOptions
  onChange: Props['onChange']
  onChooseOutputDir: () => void
}) {
  const folderName = options.outputDir
    ? options.outputDir.split(/[\\/]/).filter(Boolean).pop() ?? options.outputDir
    : 'Same as source file'

  const sameAsSource = options.overwrite && !options.outputDir && options.filenameSuffix === ''

  return (
    <section className="settings-card">
      <div className="field">
        <label className="field__label" htmlFor="suffix">
          Filename suffix
        </label>
        <input
          id="suffix"
          className="control"
          value={options.filenameSuffix}
          spellCheck={false}
          placeholder="(none)"
          onChange={(event) => onChange('filenameSuffix', event.target.value)}
        />
      </div>

      <div className="field">
        <span className="field__label">Output folder</span>
        <div className="field__row">
          <button
            type="button"
            className="control control--button"
            title={options.outputDir || 'Each image is saved next to its source'}
            onClick={onChooseOutputDir}
          >
            <FolderOpen size={15} strokeWidth={1.75} aria-hidden="true" />
            <span className="control--button__label">{folderName}</span>
          </button>
          {options.outputDir ? (
            <button
              type="button"
              className="icon-reset"
              aria-label="Save beside each source file instead"
              title="Save beside each source file instead"
              onClick={() => onChange('outputDir', '')}
            >
              <X size={14} strokeWidth={2} />
            </button>
          ) : null}
        </div>
      </div>

      <label className="check-row">
        <input
          type="checkbox"
          checked={options.overwrite}
          onChange={(event) => onChange('overwrite', event.target.checked)}
        />
        <span>
          Replace matching files
          <span className="check-row__hint">Otherwise EarthPress never overwrites anything.</span>
        </span>
      </label>

      {options.overwrite ? (
        <p className={`settings-warning ${sameAsSource ? 'is-danger' : ''}`}>
          {sameAsSource
            ? 'Same as source: your originals will be replaced and there is no copy left behind.'
            : 'Files already at the destination will be replaced.'}
        </p>
      ) : null}
    </section>
  )
}

// ---------------------------------------------------------------------------
// Panel
// ---------------------------------------------------------------------------

export function SettingsPanel({
  options,
  onChange,
  onApplyPreset,
  onChooseOutputDir,
  onConvert,
  onCancel,
  isConverting,
  count,
  totalSize,
  hasResults,
  sizeReadout,
  targetMisses,
}: Props) {
  const handleFormat = (format: ImageFormat) => {
    onChange('format', format)
    if (!FORMAT_INFO[format].lossless && options.lossless) onChange('lossless', false)
    // PNG ignores the quality dial, so a byte target has nothing to search.
    if (format === 'png') onChange('qualityMode', 'level')
  }

  return (
    <aside className="settings-panel" aria-label="Settings">
      <div className="settings-panel__scroll">
        <PresetBar options={options} onApplyPreset={onApplyPreset} />

        <FormatPicker format={options.format} onChange={handleFormat} />

        <QualityCard options={options} onChange={onChange} targetMisses={targetMisses} />

        <SizeCard options={options} onChange={onChange} sizeReadout={sizeReadout} />

        <OutputCard
          options={options}
          onChange={onChange}
          onChooseOutputDir={onChooseOutputDir}
        />
      </div>

      <div className="settings-panel__footer">
        <p className="settings-panel__summary">
          {count > 0 ? `${formatCount(count)} · ${formatBytes(totalSize)}` : 'Nothing added yet'}
        </p>

        {isConverting ? (
          <button type="button" className="convert-button is-cancel" onClick={onCancel}>
            <Loader2 size={18} strokeWidth={2} className="convert-button__spin" aria-hidden="true" />
            Cancel conversion
            <span className="convert-button__key">Esc</span>
          </button>
        ) : (
          <button
            type="button"
            className="convert-button"
            onClick={onConvert}
            disabled={count === 0}
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" />
            </svg>
            {count === 0 ? 'Convert' : hasResults ? 'Convert again' : `Convert ${formatCount(count)}`}
            <span className="convert-button__key">Ctrl ↵</span>
          </button>
        )}
      </div>
    </aside>
  )
}
