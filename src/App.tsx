import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { X } from 'lucide-react'
import { AddBar } from './components/AddBar'
import { BatchHeader } from './components/BatchHeader'
import { DropOverlay } from './components/DropOverlay'
import { ImageList } from './components/ImageList'
import { PreviewCard } from './components/PreviewCard'
import { SettingsPanel } from './components/SettingsPanel'
import { ShortcutsSheet } from './components/ShortcutsSheet'
import { SummaryCard } from './components/SummaryCard'
import { TitleBar } from './components/TitleBar'
import { useKeyboardShortcuts } from './hooks/useKeyboardShortcuts'
import { useTheme } from './hooks/useTheme'
import { formatLabel } from './utils/formats'
import { formatBytes, formatCount } from './utils/format'
import { applyPreset } from './utils/presets'
import type {
  ConversionProgress,
  ConvertOptions,
  ConvertResult,
  ImageEstimate,
  ImageItem,
  ImageTarget,
  Preset,
  SortField,
  SortOrder,
} from './types'
import './App.css'

const defaultOptions: ConvertOptions = {
  format: 'webp',
  quality: 80,
  lossless: false,
  qualityMode: 'level',
  targetBytes: 200 * 1024,
  sizeMode: 'resize',
  resizeKind: 'max',
  resizeValue: 1600,
  aspectPreset: '1:1',
  customWidth: 1080,
  customHeight: 1080,
  fitMode: 'cover',
  cropPosition: 'center',
  background: '#fffaf2',
  pngCompression: 8,
  pngPalette: false,
  overwrite: false,
  filenameSuffix: '-earthpress',
  outputDir: '',
}

const ESTIMATE_DEBOUNCE = 260

function App() {
  const [images, setImages] = useState<ImageItem[]>([])
  const [options, setOptions] = useState<ConvertOptions>(defaultOptions)
  const [results, setResults] = useState<Record<string, ConvertResult>>({})
  const [progress, setProgress] = useState<Record<string, ConversionProgress>>({})
  const [estimates, setEstimates] = useState<Record<string, ImageEstimate>>({})
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [isConverting, setIsConverting] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const { preference, selectTheme } = useTheme()
  const [showShortcuts, setShowShortcuts] = useState(false)
  const [sortField, setSortField] = useState<SortField>('added')
  const [sortOrder, setSortOrder] = useState<SortOrder>('asc')
  const [dragActiveIndex, setDragActiveIndex] = useState<number | null>(null)

  // One estimate pass at a time, always for the newest settings.
  const estimateRequest = useRef<{
    inputs: Array<Pick<ImageItem, 'id' | 'path' | 'width' | 'height'>>
    options: ConvertOptions
  } | null>(null)
  const estimateBusy = useRef(false)
  const estimateToken = useRef(0)
  /** Only estimate-relevant settings; typing a filename suffix must not re-run it. */
  const lastEstimateKey = useRef('')

  const converted = useMemo(() => Object.values(results).filter((r) => r.ok), [results])
  const hasResults = Object.keys(results).length > 0
  const totalSize = useMemo(() => images.reduce((sum, image) => sum + image.size, 0), [images])

  const mergeImages = useCallback((incoming: ImageItem[]) => {
    const valid = incoming.filter((image) => image.path && !image.error)
    const failed = incoming.filter((image) => image.error)

    setImages((current) => {
      const known = new Set(current.map((image) => image.path))
      return [...current, ...valid.filter((image) => !known.has(image.path))]
    })
    // A new file means the batch no longer matches what is on screen.
    setResults({})
    setProgress({})
    setNotice(
      failed.length > 0
        ? `${formatCount(failed.length)} could not be read and ${failed.length === 1 ? 'was' : 'were'} skipped.`
        : null,
    )
  }, [])

  const chooseImages = useCallback(async () => {
    const selected = await window.earthpress.selectImages()
    mergeImages(selected)
  }, [mergeImages])

  const chooseOutputDir = useCallback(async () => {
    const dir = await window.earthpress.selectOutputDir()
    if (dir) setOptions((current) => ({ ...current, outputDir: dir }))
  }, [])

  // The OS drag/drop listener replaces Electron's HTML5 dataTransfer handling.
  useEffect(
    () =>
      window.earthpress.subscribeDragDrop({
        onHover: setIsDragging,
        onDrop: (paths) => {
          void window.earthpress.inspectFiles(paths).then(mergeImages)
        },
      }),
    [mergeImages],
  )

  useEffect(() => {
    const unsub = window.earthpress.onConversionProgress?.((update: ConversionProgress) => {
      setProgress((prev) => ({ ...prev, [update.id]: update }))
    })
    return () => unsub?.()
  }, [])

  // The selected row is derived at render time, so removing files never leaves
  // a dangling selection behind.

  /**
   * Drains queued estimate requests, one pass at a time.
   *
   * A pass that is already running keeps its results; anything queued while it
   * runs is picked up straight after, so a slider drag ends with the numbers
   * matching the final position rather than the first one.
   */
  const pumpEstimates = useCallback(async () => {
    if (estimateBusy.current) return
    estimateBusy.current = true

    try {
      while (estimateRequest.current) {
        const request = estimateRequest.current
        estimateRequest.current = null
        const token = ++estimateToken.current

        try {
          const result = await window.earthpress.estimateImages(request.inputs, request.options)
          if (token === estimateToken.current) {
            setEstimates(Object.fromEntries(result.map((estimate) => [estimate.id, estimate])))
          }
        } catch {
          // Estimates are a convenience. Rows simply keep whatever they had.
          return
        }
      }
    } finally {
      estimateBusy.current = false
    }
  }, [])

  useEffect(() => {
    if (images.length === 0) {
      estimateRequest.current = null
      return
    }

    // Never compete with a conversion for the CPU.
    if (isConverting) return

    const key = JSON.stringify([
      images.map((image) => [image.id, image.width, image.height]),
      options.format,
      options.quality,
      options.lossless,
      options.qualityMode,
      options.targetBytes,
      options.sizeMode,
      options.resizeKind,
      options.resizeValue,
      options.aspectPreset,
      options.customWidth,
      options.customHeight,
      options.fitMode,
      options.cropPosition,
      options.background,
      options.pngCompression,
      options.pngPalette,
    ])

    if (lastEstimateKey.current === key) return
    lastEstimateKey.current = key

    const inputs = images.map(({ id, path, width, height }) => ({ id, path, width, height }))
    const timer = window.setTimeout(() => {
      estimateRequest.current = { inputs, options }
      void pumpEstimates()
    }, ESTIMATE_DEBOUNCE)

    return () => window.clearTimeout(timer)
  }, [images, isConverting, options, pumpEstimates])

  /** Target geometry, as measured by the estimator. */
  const targets = useMemo(() => {
    const map: Record<string, ImageTarget> = {}
    for (const [id, estimate] of Object.entries(estimates)) {
      map[id] = { id, width: estimate.outputWidth, height: estimate.outputHeight }
    }
    return map
  }, [estimates])

  const targetMisses = useMemo(
    () => Object.values(estimates).filter((estimate) => !estimate.meetsTarget).length,
    [estimates],
  )

  const convert = useCallback(async () => {
    if (images.length === 0) return

    setIsConverting(true)
    setProgress({})
    setResults({})
    setNotice(null)

    try {
      const conversionResults = await window.earthpress.convertImages(images, options)
      const nextResults = Object.fromEntries(
        conversionResults.map((result) => [result.id, result]),
      )
      setResults(nextResults)

      const failed = conversionResults.filter((result) => !result.ok).length
      setNotice(failed > 0 ? `${formatCount(failed)} could not be written. Retry from the list.` : null)

      // A long batch deserves a nudge when the user has walked away.
      const converted = conversionResults.filter((result) => result.ok)
      const saved = converted.reduce(
        (total, result) => total + Math.max(0, result.savedBytes ?? 0),
        0,
      )
      void window.earthpress.notifyBatchFinished?.(
        failed > 0 ? 'Batch finished with problems' : 'Batch complete',
        failed > 0
          ? `${formatCount(converted.length)} converted, ${formatCount(failed)} could not be written.`
          : `${formatCount(converted.length)} converted · saved ${formatBytes(saved)}.`,
      )
    } catch (error) {
      // Tauri rejects `invoke` with the raw string our command returned, so
      // unwrap both shapes rather than dropping the backend's message.
      const detail =
        error instanceof Error
          ? error.message
          : typeof error === 'string' && error
            ? error
            : 'Conversion failed.'
      setNotice(detail)
    } finally {
      setProgress({})
      setIsConverting(false)
    }
  }, [images, options])

  const cancelConversion = useCallback(async () => {
    await window.earthpress.cancelConversion()
    setIsConverting(false)
    setProgress({})
    setNotice('Conversion cancelled. The files already written are still there.')
  }, [])

  const retryImage = useCallback(
    async (id: string) => {
      const image = images.find((item) => item.id === id)
      if (!image || isConverting) return

      setIsConverting(true)
      setNotice(null)

      try {
        const [result] = await window.earthpress.convertImages([image], options)
        if (result) setResults((current) => ({ ...current, [result.id]: result }))
      } catch (error) {
        setNotice(typeof error === 'string' ? error : 'Conversion failed.')
      } finally {
        setProgress({})
        setIsConverting(false)
      }
    },
    [images, isConverting, options],
  )

  /** Back to editing. Output files stay exactly where they are. */
  const undoConvert = useCallback(() => {
    setResults({})
    setProgress({})
    setNotice(null)
  }, [])

  const removeImage = useCallback((id: string) => {
    setImages((current) => current.filter((image) => image.id !== id))
    setResults((current) => {
      const next = { ...current }
      delete next[id]
      return next
    })
  }, [])

  const clearAll = useCallback(() => {
    setImages([])
    setResults({})
    setProgress({})
    setNotice(null)
  }, [])

  const updateOption = useCallback(
    <K extends keyof ConvertOptions>(key: K, value: ConvertOptions[K]) => {
      setOptions((current) => ({ ...current, [key]: value }))
    },
    [],
  )

  const applySavedPreset = useCallback((preset: Preset) => {
    setOptions((current) => applyPreset(preset, current))
  }, [])

  const openOutputFolder = useCallback(() => {
    const target = options.outputDir || converted[0]?.outputPath
    if (target) void window.earthpress.revealPath(target)
  }, [converted, options.outputDir])

  // Sorting. "Added" keeps the order the files came in, which is also the order
  // drag-to-reorder works on.
  const sortedImages = useMemo(() => {
    if (sortField === 'added') return images

    const direction = sortOrder === 'asc' ? 1 : -1
    return [...images].sort((a, b) => {
      if (sortField === 'name') return a.name.localeCompare(b.name) * direction
      if (sortField === 'size') return (a.size - b.size) * direction
      if (sortField === 'width') return (a.width - b.width) * direction
      if (sortField === 'height') return (a.height - b.height) * direction
      return a.format.localeCompare(b.format) * direction
    })
  }, [images, sortField, sortOrder])

  const handleSortChange = useCallback((field: SortField) => {
    setSortField((previous) => {
      if (previous === field) {
        setSortOrder((order) => (order === 'asc' ? 'desc' : 'asc'))
        return previous
      }
      setSortOrder('asc')
      return field
    })
  }, [])

  const handleReorder = useCallback((fromIndex: number, toIndex: number) => {
    setImages((current) => {
      const next = [...current]
      const [moved] = next.splice(fromIndex, 1)
      next.splice(toIndex, 0, moved)
      return next
    })
    setDragActiveIndex(toIndex)
  }, [])

  const selectedImage = useMemo(
    () => images.find((image) => image.id === selectedId) ?? images[0] ?? null,
    [images, selectedId],
  )

  const sizeReadout = useMemo(() => {
    if (!selectedImage) return null
    const target = targets[selectedImage.id]
    return target ? `${target.width} × ${target.height} px` : null
  }, [selectedImage, targets])

  useKeyboardShortcuts({
    'ctrl+o': chooseImages,
    'ctrl+shift+o': chooseOutputDir,
    'ctrl+enter': isConverting ? undefined : convert,
    '?': () => setShowShortcuts(true),
    escape: () => {
      if (showShortcuts) {
        setShowShortcuts(false)
        return
      }
      if (isConverting) void cancelConversion()
    },
    // The hook maps both Delete and Ctrl+D onto the same handler.
    delete: clearAll,
    'ctrl+d': clearAll,
  })

  const outputLabel = formatLabel(options.format)

  return (
    <main className="app-shell">
      <TitleBar
        images={images}
        results={results}
        estimates={estimates}
        preference={preference}
        onSelectTheme={selectTheme}
        onShowShortcuts={() => setShowShortcuts(true)}
      />

      <DropOverlay active={isDragging} />
      <ShortcutsSheet open={showShortcuts} onClose={() => setShowShortcuts(false)} />

      <section className="workspace">
        <section className="image-area" aria-label="Batch">
          <AddBar onChooseImages={chooseImages} />

          {notice ? (
            <div className="notice" role="status">
              <span className="notice__text">{notice}</span>
              <button
                type="button"
                className="notice__close"
                aria-label="Dismiss message"
                onClick={() => setNotice(null)}
              >
                <X size={13} strokeWidth={2} />
              </button>
            </div>
          ) : null}

          <BatchHeader
            count={images.length}
            totalSize={totalSize}
            sortField={sortField}
            sortOrder={sortOrder}
            onSortChange={handleSortChange}
            onClear={clearAll}
            disabled={images.length === 0}
          />

          <ImageList
            images={sortedImages}
            results={results}
            progress={progress}
            estimates={estimates}
            targets={targets}
            selectedId={selectedImage?.id ?? null}
            outputLabel={outputLabel}
            isConverting={isConverting}
            reorderEnabled={sortField === 'added' && !isConverting}
            onSelect={setSelectedId}
            onRemove={removeImage}
            onRetry={(id) => void retryImage(id)}
            onReveal={(path) => void window.earthpress.revealPath(path)}
            onReorder={handleReorder}
            dragActiveIndex={dragActiveIndex}
            onDragStart={setDragActiveIndex}
            onDragEnd={() => setDragActiveIndex(null)}
          />

          {images.length > 0 && selectedImage ? (
            hasResults ? (
              <SummaryCard
                images={images}
                results={results}
                options={options}
                formatLabel={outputLabel}
                onOpenFolder={openOutputFolder}
                onUndo={undoConvert}
              />
            ) : (
              <PreviewCard
                image={selectedImage}
                target={targets[selectedImage.id]}
                estimate={estimates[selectedImage.id]}
                options={options}
              />
            )
          ) : null}
        </section>

        <SettingsPanel
          options={options}
          onChange={updateOption}
          onApplyPreset={applySavedPreset}
          onChooseOutputDir={chooseOutputDir}
          onConvert={() => void convert()}
          onCancel={() => void cancelConversion()}
          isConverting={isConverting}
          count={images.length}
          totalSize={totalSize}
          hasResults={hasResults}
          sizeReadout={sizeReadout}
          targetMisses={targetMisses}
        />
      </section>
    </main>
  )
}

export default App
