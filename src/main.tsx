import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { ErrorBoundary } from './components/ErrorBoundary'
import { installBridge, readThemePreference, resolveTheme } from './bridge'
import App from './App'

// Bundled, so the reworked type stays available with no network - the app
// promises to be 100% offline. Both families are OFL-1.1 (see NOTICES).
import './fonts.css'
import './index.css'

installBridge()

// Apply the stored theme before React paints, so following a dark OS never
// flashes a light frame first.
document.documentElement.setAttribute('data-theme', resolveTheme(readThemePreference()))

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
)
