import { useCallback, useEffect, useState } from 'react'
import {
  readThemePreference,
  resolveTheme,
  setNativeTheme,
  storeThemePreference,
  watchSystemTheme,
} from '../bridge'
import type { ResolvedTheme, ThemePreference } from '../types'

/** Matches the transition window in `index.css` (`.theme-transition`). */
const TRANSITION_MS = 200

/**
 * Three-way theme: light, dark, or follow the OS.
 *
 * The preference is what the user picked and is what persists; the resolved
 * theme is what the document actually renders. `main.tsx` applies the resolved
 * theme before the first paint so a dark OS never flashes light.
 */
export function useTheme() {
  const [preference, setPreference] = useState<ThemePreference>(() => readThemePreference())
  const [theme, setTheme] = useState<ResolvedTheme>(() => resolveTheme(readThemePreference()))

  // While the preference is "system", keep following the OS live.
  useEffect(() => {
    if (preference !== 'system') return
    return watchSystemTheme(setTheme)
  }, [preference])

  useEffect(() => {
    const root = document.documentElement
    const isChange = root.getAttribute('data-theme') !== null && root.getAttribute('data-theme') !== theme

    root.setAttribute('data-theme', theme)
    void setNativeTheme(theme)

    if (!isChange) return

    root.classList.add('theme-transition')
    const timer = window.setTimeout(() => root.classList.remove('theme-transition'), TRANSITION_MS)

    return () => {
      window.clearTimeout(timer)
      root.classList.remove('theme-transition')
    }
  }, [theme])

  const selectTheme = useCallback((next: ThemePreference) => {
    setPreference(next)
    storeThemePreference(next)
    setTheme(resolveTheme(next))
  }, [])

  return { preference, theme, isDark: theme === 'dark', selectTheme }
}
