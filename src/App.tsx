import { useMemo, useState } from 'react'
import {
  CheckCircle2,
  ChevronDown,
  ChevronsUpDown,
  Download,
  FolderOpen,
  FolderSearch,
  ImagePlus,
  Leaf,
  Loader2,
  RefreshCw,
  SlidersHorizontal,
  Trash2,
  XCircle,
} from 'lucide-react'
import './App.css'
import type {
  AspectPreset,
  ConvertOptions,
  ConvertResult,
  CropPosition,
  FitMode,
  ImageFormat,
  ImageItem,
  ResizeKind,
  SizeMode,
} from './types'

const defaultOptions: ConvertOptions = {
  format: 'webp',
  quality: 82,
  lossless: false,
  sizeMode: 'original',
  resizeKind: 'max',
  resizeMode: 'original',
  resizeValue: 1600,
  aspectPreset: '1:1',
  customWidth: 1080,
  customHeight: 1080,
  fitMode: 'cover',
  cropPosition: 'center',
  background: '#fffaf2',
  allowUpscale: false,
  pngCompression: 8,
  pngPalette: false,
  preserveMetadata: false,
  overwrite: false,
  filenameSuffix: '-earthpress',
  outputDir: '',
}

const aspectPresets: Array<{ value: AspectPreset; label: string; hint: string }> = [
  { value: '1:1', label: 'Square', hint: '1:1' },
  { value: '4:5', label: 'Portrait', hint: '4:5' },
  { value: '5:4', label: 'Frame', hint: '5:4' },
  { value: '4:3', label: 'Classic', hint: '4:3' },
  { value: '3:2', label: 'Photo', hint: '3:2' },
  { value: '16:9', label: 'Wide', hint: '16:9' },
  { value: '9:16', label: 'Story', hint: '9:16' },
]

function RatioIcon({ ratio }: { ratio: AspectPreset }) {
  const [width, height] = ratio.split(':').map(Number)
  const landscape = width >= height
  const iconWidth = landscape ? 30 : Math.max(14, Math.round((width / height) * 30))
  const iconHeight = landscape ? Math.max(14, Math.round((height / width) * 30)) : 30

  return (
    <span className="ratio-icon" aria-hidden="true">
      <span style={{ width: iconWidth, height: iconHeight }} />
    </span>
  )
}

function formatBytes(bytes = 0) {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let size = bytes / 1024
  let unit = units[0]
  for (let index = 1; index < units.length && size >= 1024; index += 1) {
    size /= 1024
    unit = units[index]
  }
  return `${size.toFixed(size >= 10 ? 1 : 2)} ${unit}`
}

function formatName(format: ImageFormat) {
  return format === 'jpg' ? 'JPG' : format.toUpperCase()
}

function App() {
  const [images, setImages] = useState<ImageItem[]>([])
  const [options, setOptions] = useState<ConvertOptions>(defaultOptions)
  const [results, setResults] = useState<Record<string, ConvertResult>>({})
  const [isDragging, setIsDragging] = useState(false)
  const [isConverting, setIsConverting] = useState(false)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [message, setMessage] = useState('Add images to begin.')

  const totalSize = useMemo(() => images.reduce((sum, image) => sum + image.size, 0), [images])
  const converted = Object.values(results).filter((result) => result.ok)
  const savedTotal = converted.reduce((sum, result) => sum + (result.savedBytes || 0), 0)
  const sizeSummary =
    options.sizeMode === 'custom'
      ? `${options.customWidth} x ${options.customHeight}`
      : options.sizeMode === 'aspect'
        ? options.aspectPreset
        : options.sizeMode === 'resize'
          ? `${options.resizeKind} ${options.resizeValue}`
          : 'Original'

  const mergeImages = (incoming: ImageItem[]) => {
    const valid = incoming.filter((image) => image.path && !image.error)
    const failed = incoming.filter((image) => image.error)
    setImages((current) => {
      const known = new Set(current.map((image) => image.path))
      return [...current, ...valid.filter((image) => !known.has(image.path))]
    })
    setResults({})
    if (failed.length > 0) {
      setMessage(`${valid.length} added. ${failed.length} file could not be read.`)
    } else if (valid.length > 0) {
      setMessage(`${valid.length} image${valid.length === 1 ? '' : 's'} added.`)
    }
  }

  const chooseImages = async () => {
    const selected = await window.earthpress.selectImages()
    mergeImages(selected)
  }

  const chooseOutputDir = async () => {
    const dir = await window.earthpress.selectOutputDir()
    if (dir) setOptions((current) => ({ ...current, outputDir: dir }))
  }

  const onDrop = async (event: React.DragEvent<HTMLElement>) => {
    event.preventDefault()
    setIsDragging(false)
    const paths = [...event.dataTransfer.files]
      .map((file) => window.earthpress.getPathForFile(file))
      .filter(Boolean)
    if (paths.length === 0) return
    const inspected = await window.earthpress.inspectFiles(paths)
    mergeImages(inspected)
  }

  const convert = async () => {
    if (images.length === 0) {
      setMessage('Add at least one image first.')
      return
    }
    if (!options.outputDir) {
      setMessage('Choose an output folder before converting.')
      return
    }

    setIsConverting(true)
    setMessage(`Converting ${images.length} image${images.length === 1 ? '' : 's'}...`)
    const conversionResults = await window.earthpress.convertImages(images, options)
    const nextResults = Object.fromEntries(conversionResults.map((result) => [result.id, result]))
    setResults(nextResults)
    setIsConverting(false)

    const okCount = conversionResults.filter((result) => result.ok).length
    setMessage(`${okCount} of ${conversionResults.length} image${conversionResults.length === 1 ? '' : 's'} converted.`)
  }

  const removeImage = (id: string) => {
    setImages((current) => current.filter((image) => image.id !== id))
    setResults((current) => {
      const next = { ...current }
      delete next[id]
      return next
    })
  }

  const clearAll = () => {
    setImages([])
    setResults({})
    setMessage('Workspace cleared.')
  }

  const updateOption = <K extends keyof ConvertOptions>(key: K, value: ConvertOptions[K]) => {
    setOptions((current) => ({ ...current, [key]: value }))
  }

  return (
    <main
      className="app-shell"
      onDragOver={(event) => {
        event.preventDefault()
        setIsDragging(true)
      }}
      onDragLeave={() => setIsDragging(false)}
      onDrop={onDrop}
    >
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <Leaf size={22} />
          </span>
          <div>
            <h1>EarthPress</h1>
            <p>Local image conversion and compression for Windows</p>
          </div>
        </div>
        <div className="summary-strip" aria-label="Batch summary">
          <span>{images.length} files</span>
          <span>{formatBytes(totalSize)}</span>
          <span>{savedTotal > 0 ? `${formatBytes(savedTotal)} saved` : 'Ready offline'}</span>
        </div>
      </header>

      <section className="workspace">
        <section className="image-area" aria-label="Images">
          <div className={`drop-zone ${isDragging ? 'is-dragging' : ''}`}>
            <div>
              <ImagePlus size={30} />
              <h2>Drop images here</h2>
              <p>PNG, JPG, WebP, AVIF, TIFF, and BMP are welcome.</p>
            </div>
            <button type="button" className="primary-button" onClick={chooseImages}>
              <ImagePlus size={18} />
              Add images
            </button>
          </div>

          <div className="list-toolbar">
            <div>
              <strong>Batch</strong>
              <span>{message}</span>
            </div>
            <button type="button" className="ghost-button" onClick={clearAll} disabled={images.length === 0}>
              <Trash2 size={16} />
              Clear
            </button>
          </div>

          <div className="image-list">
            {images.length === 0 ? (
              <div className="empty-state">
                <ImagePlus size={42} />
                <p>Your images will appear here with size, dimensions, and conversion status.</p>
              </div>
            ) : (
              images.map((image) => {
                const result = results[image.id]
                return (
                  <article className="image-row" key={image.id}>
                    <img src={image.preview} alt="" />
                    <div className="image-meta">
                      <strong title={image.name}>{image.name}</strong>
                      <span>
                        {image.width} x {image.height} · {image.format.toUpperCase()} · {formatBytes(image.size)}
                      </span>
                      {result?.ok && <span className="output-note">{formatBytes(result.outputSize)} output</span>}
                      {result && !result.ok && <span className="error-text">{result.error}</span>}
                    </div>
                    <div className="row-actions">
                      {result?.ok && <CheckCircle2 className="status-ok" size={22} aria-label="Converted" />}
                      {result && !result.ok && <XCircle className="status-error" size={22} aria-label="Failed" />}
                      {result?.ok && (
                        <button
                          type="button"
                          className="icon-button folder-icon"
                          onClick={() => result.outputPath && window.earthpress.revealPath(result.outputPath)}
                          aria-label="Show converted image in folder"
                          title="Show in folder"
                        >
                          <FolderSearch size={16} />
                        </button>
                      )}
                      <button type="button" className="icon-button" onClick={() => removeImage(image.id)} aria-label="Remove image">
                        <Trash2 size={16} />
                      </button>
                    </div>
                  </article>
                )
              })
            )}
          </div>
        </section>

        <aside className="settings-panel" aria-label="Conversion settings">
          <div className="panel-heading">
            <SlidersHorizontal size={20} />
            <h2>Settings</h2>
          </div>

          <label className="field">
            <span>Output format</span>
            <div className="segmented">
              {(['webp', 'jpg', 'png', 'avif'] as ImageFormat[]).map((format) => (
                <button
                  type="button"
                  key={format}
                  className={options.format === format ? 'active' : ''}
                  onClick={() => updateOption('format', format)}
                >
                  {formatName(format)}
                </button>
              ))}
            </div>
          </label>

          <div className="settings-group">
            <div className="group-title">
              <span>Quality</span>
              <strong>{options.format === 'png' ? 'PNG' : options.lossless ? 'Lossless' : options.quality}</strong>
            </div>
            <div className="range-line compact-range">
              <input
                type="range"
                min="1"
                max="100"
                value={options.quality}
                disabled={options.format === 'png' || options.lossless}
                onChange={(event) => updateOption('quality', Number(event.target.value))}
              />
              <strong>{options.quality}</strong>
            </div>
            <label className="toggle-line compact-toggle">
              <input
                type="checkbox"
                checked={options.lossless}
                disabled={options.format === 'jpg' || options.format === 'png'}
                onChange={(event) => updateOption('lossless', event.target.checked)}
              />
              <span>Lossless WebP/AVIF</span>
            </label>
          </div>

          <div className="settings-group">
            <div className="group-title">
              <span>Size</span>
              <strong>{sizeSummary}</strong>
            </div>

            <div className="segmented four-way">
              {[
                ['original', 'Original'],
                ['resize', 'Resize'],
                ['aspect', 'Ratio'],
                ['custom', 'Custom'],
              ].map(([mode, label]) => (
                <button
                  type="button"
                  key={mode}
                  className={options.sizeMode === mode ? 'active' : ''}
                  onClick={() => updateOption('sizeMode', mode as SizeMode)}
                >
                  {label}
                </button>
              ))}
            </div>

            {options.sizeMode === 'resize' && (
              <div className="two-col uneven">
                <label className="field">
                  <span>Resize by</span>
                  <div className="select-wrap">
                    <select value={options.resizeKind} onChange={(event) => updateOption('resizeKind', event.target.value as ResizeKind)}>
                      <option value="max">Max side</option>
                      <option value="width">Width</option>
                      <option value="height">Height</option>
                      <option value="percent">Percent</option>
                    </select>
                    <ChevronsUpDown size={16} />
                  </div>
                </label>
                <label className="field">
                  <span>{options.resizeKind === 'percent' ? 'Scale' : 'Pixels'}</span>
                  <input
                    type="number"
                    min="1"
                    value={options.resizeValue}
                    onChange={(event) => updateOption('resizeValue', Number(event.target.value))}
                  />
                </label>
              </div>
            )}

            {options.sizeMode === 'aspect' && (
              <div className="preset-grid" aria-label="Aspect ratio presets">
                {aspectPresets.map((preset) => (
                  <button
                    type="button"
                    key={preset.value}
                    className={options.aspectPreset === preset.value ? 'active' : ''}
                    onClick={() => updateOption('aspectPreset', preset.value)}
                    title={`${preset.label} ${preset.hint}`}
                  >
                    <RatioIcon ratio={preset.value} />
                    <span>{preset.hint}</span>
                  </button>
                ))}
              </div>
            )}

            {options.sizeMode === 'custom' && (
              <div className="two-col">
                <label className="field">
                  <span>Width</span>
                  <input
                    type="number"
                    min="1"
                    value={options.customWidth}
                    onChange={(event) => updateOption('customWidth', Number(event.target.value))}
                  />
                </label>
                <label className="field">
                  <span>Height</span>
                  <input
                    type="number"
                    min="1"
                    value={options.customHeight}
                    onChange={(event) => updateOption('customHeight', Number(event.target.value))}
                  />
                </label>
              </div>
            )}

            {(options.sizeMode === 'aspect' || options.sizeMode === 'custom') && (
              <>
                <label className="field">
                  <span>Object fit</span>
                  <div className="select-wrap">
                    <select value={options.fitMode} onChange={(event) => updateOption('fitMode', event.target.value as FitMode)}>
                      <option value="cover">Cover - crop edges</option>
                      <option value="contain">Contain - add background</option>
                      <option value="fill">Fill - stretch to size</option>
                      <option value="inside">Inside - fit within size</option>
                      <option value="outside">Outside - fill beyond size</option>
                    </select>
                    <ChevronsUpDown size={16} />
                  </div>
                </label>

                <div className="two-col">
                  <label className="field">
                    <span>Focus</span>
                    <div className="select-wrap">
                      <select
                        value={options.cropPosition}
                        onChange={(event) => updateOption('cropPosition', event.target.value as CropPosition)}
                      >
                        <option value="center">Center</option>
                        <option value="attention">Smart attention</option>
                        <option value="entropy">Smart detail</option>
                        <option value="top">Top</option>
                        <option value="right">Right</option>
                        <option value="bottom">Bottom</option>
                        <option value="left">Left</option>
                      </select>
                      <ChevronsUpDown size={16} />
                    </div>
                  </label>
                  {options.fitMode === 'contain' && (
                    <label className="field">
                      <span>Background</span>
                      <input
                        type="color"
                        value={options.background}
                        onChange={(event) => updateOption('background', event.target.value)}
                        aria-label="Background color for contain fit"
                      />
                    </label>
                  )}
                </div>
              </>
            )}
          </div>

          <div className={`advanced-panel ${advancedOpen ? 'open' : ''}`}>
            <button type="button" className="advanced-toggle" onClick={() => setAdvancedOpen((open) => !open)}>
              <span>Advanced</span>
              <ChevronDown size={17} />
            </button>

            {advancedOpen && (
              <div className="advanced-body">
                <label className="toggle-line">
                  <input
                    type="checkbox"
                    checked={options.allowUpscale}
                    onChange={(event) => updateOption('allowUpscale', event.target.checked)}
                  />
                  <span>Allow enlargement</span>
                </label>

                <label className="toggle-line">
                  <input
                    type="checkbox"
                    checked={options.pngPalette}
                    disabled={options.format !== 'png'}
                    onChange={(event) => updateOption('pngPalette', event.target.checked)}
                  />
                  <span>Indexed PNG palette</span>
                </label>

                <label className="toggle-line">
                  <input
                    type="checkbox"
                    checked={options.preserveMetadata}
                    onChange={(event) => updateOption('preserveMetadata', event.target.checked)}
                  />
                  <span>Preserve metadata</span>
                </label>

                <label className="toggle-line">
                  <input
                    type="checkbox"
                    checked={options.overwrite}
                    onChange={(event) => updateOption('overwrite', event.target.checked)}
                  />
                  <span>Overwrite existing files</span>
                </label>

                <label className="field">
                  <span>Filename suffix</span>
                  <input
                    type="text"
                    value={options.filenameSuffix}
                    placeholder="-earthpress"
                    onChange={(event) => updateOption('filenameSuffix', event.target.value)}
                  />
                </label>
              </div>
            )}
          </div>

          <div className="sticky-actions">
            <div className="output-picker">
              <span>Output folder</span>
              <button type="button" className="folder-button" onClick={chooseOutputDir}>
                <FolderOpen size={17} />
                {options.outputDir || 'Choose folder'}
              </button>
            </div>

            <button type="button" className="convert-button" onClick={convert} disabled={isConverting}>
              {isConverting ? <Loader2 className="spin" size={19} /> : <Download size={19} />}
              Convert batch
            </button>

            {converted.length > 0 && (
              <button
                type="button"
                className="secondary-button"
                onClick={() => converted[0].outputPath && window.earthpress.revealPath(converted[0].outputPath)}
              >
                <RefreshCw size={16} />
                Open latest output
              </button>
            )}
          </div>
        </aside>
      </section>
    </main>
  )
}

export default App
