/**
 * Tauri bridge.
 *
 * The React UI talks to `window.earthpress`, exactly like the original
 * Electron build did. This module installs that object on top of the Tauri
 * IPC layer so the components do not need to know which runtime they run on.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-dialog'
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import type {
  ConvertOptions,
  ConvertResult,
  ConversionProgress,
  DragDropHandlers,
  ImageEstimate,
  ImageItem,
  PreviewPayload,
  ResolvedTheme,
  ThemePreference,
} from './types'

const IMAGE_EXTENSIONS = ['jpg', 'jpeg', 'png', 'webp', 'avif', 'tif', 'tiff', 'bmp']
const THEME_STORAGE_KEY = 'earthpress-theme'

/** Asks the OS for images, then inspects whatever the user picked. */
async function selectImages(): Promise<ImageItem[]> {
  const selection = await open({
    title: 'Choose images',
    multiple: true,
    directory: false,
    filters: [{ name: 'Images', extensions: IMAGE_EXTENSIONS }],
  })

  if (!selection) return []
  const paths = Array.isArray(selection) ? selection : [selection]
  if (paths.length === 0) return []

  return inspectFiles(paths)
}

function inspectFiles(filePaths: string[]): Promise<ImageItem[]> {
  return invoke<ImageItem[]>('inspect_files', { filePaths })
}

async function selectOutputDir(): Promise<string | null> {
  const selection = await open({
    title: 'Choose output folder',
    directory: true,
    multiple: false,
  })

  if (!selection) return null
  return Array.isArray(selection) ? (selection[0] ?? null) : selection
}

function convertImages(files: ImageItem[], options: ConvertOptions): Promise<ConvertResult[]> {
  return invoke<ConvertResult[]>('convert_images', { files, options })
}

/** Both halves of the preview card for one file, at display size. */
function previewPair(filePath: string, options: ConvertOptions): Promise<PreviewPayload> {
  return invoke<PreviewPayload>('preview_pair', { filePath, options })
}

/** What the batch will weigh with these settings. */
function estimateImages(
  inputs: Array<Pick<ImageItem, 'id' | 'path' | 'width' | 'height'>>,
  options: ConvertOptions,
): Promise<ImageEstimate[]> {
  return invoke<ImageEstimate[]>('estimate_images', { inputs, options })
}

/**
 * Reports a finished batch to the OS, but only when the user is looking
 * elsewhere.
 *
 * Permission prompts, focus checks and platform quirks are all best-effort: a
 * notification is a courtesy, never a reason for a conversion to fail.
 */
async function notifyBatchFinished(title: string, body: string): Promise<void> {
  try {
    if (await getCurrentWindow().isFocused()) return

    let granted = await isPermissionGranted()
    if (!granted) granted = (await requestPermission()) === 'granted'
    if (!granted) return

    sendNotification({ title, body })
  } catch {
    // Windows without an app identity, a denied permission, a missing
    // notification service: none of it should surface to the user.
  }
}

async function cancelConversion(): Promise<void> {
  await invoke('cancel_conversion')
}

async function revealPath(filePath: string): Promise<boolean> {
  if (!filePath) return false
  await revealItemInDir(filePath)
  return true
}

/**
 * Subscribes to per-image progress events emitted by the Rust batch worker.
 * Returns a synchronous unsubscribe function, matching the old preload API.
 */
function onConversionProgress(callback: (progress: ConversionProgress) => void): () => void {
  const pending = listen<ConversionProgress>('convert-progress', (event) => {
    callback(event.payload)
  }).catch(() => () => {})

  return () => {
    void pending.then((unlisten) => unlisten())
  }
}

/**
 * OS-level file drag and drop.
 *
 * Tauri intercepts the native drop and hands us real absolute paths, so there
 * is no equivalent of Electron's `webUtils.getPathForFile` any more.
 */
function subscribeDragDrop({ onDrop, onHover }: DragDropHandlers): () => void {
  let pending: Promise<() => void>

  try {
    pending = getCurrentWindow()
      .onDragDropEvent((event) => {
        const payload = event.payload

        switch (payload.type) {
          case 'enter':
            onHover(true)
            break
          case 'over':
            onHover(true)
            break
          case 'drop':
            onHover(false)
            if (payload.paths.length > 0) onDrop(payload.paths)
            break
          case 'leave':
            onHover(false)
            break
        }
      })
      .catch(() => () => {})
  } catch {
    // No Tauri runtime; nothing will ever drop.
    return () => {}
  }

  return () => {
    void pending.then((unlisten) => unlisten())
  }
}

/** Keeps the native chrome (dialogs, context menus) in step with the app theme. */
async function setNativeTheme(theme: ResolvedTheme): Promise<void> {
  try {
    await getCurrentWindow().setTheme(theme)
  } catch {
    // Non-fatal: some platforms refuse to restyle the chrome.
  }
}

/**
 * Custom title-bar controls.
 *
 * The window is undecorated so the app can draw the mockup's title bar itself.
 * These wrap the Tauri window API and stay quiet when the UI runs outside
 * Tauri, where there is no window to control.
 */
export const windowControls = {
  async minimize(): Promise<void> {
    try {
      await getCurrentWindow().minimize()
    } catch {
      // No native window (browser harness).
    }
  },

  async toggleMaximize(): Promise<void> {
    try {
      await getCurrentWindow().toggleMaximize()
    } catch {
      // No native window.
    }
  },

  async close(): Promise<void> {
    try {
      await getCurrentWindow().close()
    } catch {
      // No native window.
    }
  },

  async isMaximized(): Promise<boolean> {
    try {
      return await getCurrentWindow().isMaximized()
    } catch {
      return false
    }
  },

  /** Fires on every resize, because maximizing and restoring both resize. */
  onResized(callback: () => void): () => void {
    // `getCurrentWindow()` throws synchronously when there is no Tauri runtime
    // (for example when the UI is opened in a plain browser), so it cannot be
    // left inside the promise chain.
    let pending: Promise<() => void>

    try {
      pending = getCurrentWindow()
        .onResized(() => callback())
        .catch(() => () => {})
    } catch {
      return () => {}
    }

    return () => {
      void pending.then((unlisten) => unlisten())
    }
  },
}

function readThemePreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY)
    if (stored === 'dark' || stored === 'light' || stored === 'system') return stored
  } catch {
    // localStorage can be unavailable; fall through to the OS preference.
  }

  return 'system'
}

function storeThemePreference(preference: ThemePreference): void {
  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference)
  } catch {
    // Ignore storage failures.
  }
}

function systemTheme(): ResolvedTheme {
  return window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function resolveTheme(preference: ThemePreference): ResolvedTheme {
  return preference === 'system' ? systemTheme() : preference
}

/** Follows the OS while the preference is "system". */
function watchSystemTheme(callback: (theme: ResolvedTheme) => void): () => void {
  const query = window.matchMedia?.('(prefers-color-scheme: dark)')
  if (!query) return () => {}

  const handler = (event: MediaQueryListEvent) => callback(event.matches ? 'dark' : 'light')
  query.addEventListener('change', handler)

  return () => query.removeEventListener('change', handler)
}

/** Installs the legacy `window.earthpress` surface. Safe to call more than once. */
export function installBridge(): void {
  window.earthpress = {
    selectImages,
    inspectFiles,
    selectOutputDir,
    convertImages,
    cancelConversion,
    previewPair,
    estimateImages,
    notifyBatchFinished,
    revealPath,
    onConversionProgress,
    subscribeDragDrop,
    setNativeTheme,
  }
}

export {
  readThemePreference,
  storeThemePreference,
  resolveTheme,
  watchSystemTheme,
  setNativeTheme,
  subscribeDragDrop,
}
