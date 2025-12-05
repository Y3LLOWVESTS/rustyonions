// crates/svc-admin/ui/src/theme/ThemeProvider.tsx

/**
 * RO:WHAT — Theme context + provider for the svc-admin SPA.
 * RO:WHY — Centralizes theme selection and keeps it driven by backend
 *          config (`UiConfigDto.defaultTheme`) with a simple React
 *          context for components to consume.
 * RO:INTERACTS — api/adminClient.ts (getUiConfig),
 *                theme/themes.ts (actual theme tokens),
 *                layout components and top bar toggles.
 * RO:INVARIANTS — Only themes defined in `themes` are allowed. The
 *                 <html> element receives `data-theme=<name>` so CSS
 *                 can react via attribute selectors.
 */

import React, { createContext, useContext, useEffect, useState } from 'react'
import { themes } from './themes'
import { adminClient } from '../api/adminClient'

export type ThemeName = keyof typeof themes

type ThemeContextValue = {
  theme: ThemeName
  setTheme: (theme: ThemeName) => void
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

const DEFAULT_THEME: ThemeName = 'light'

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState<ThemeName>(DEFAULT_THEME)

  // Apply theme to the <html> element so CSS can react via [data-theme] selectors.
  useEffect(() => {
    document.documentElement.dataset.theme = theme
  }, [theme])

  // Fetch UI config from the backend and apply the default theme if it is known.
  useEffect(() => {
    let cancelled = false

    adminClient
      .getUiConfig()
      .then((cfg) => {
        if (cancelled) return

        const candidate = cfg.defaultTheme as ThemeName | undefined

        if (candidate && candidate in themes) {
          setTheme(candidate)
        }
      })
      .catch(() => {
        // Developer preview: if /api/ui-config fails, we stay on the local default theme.
      })

    return () => {
      cancelled = true
    }
  }, [])

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext)
  if (!ctx) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return ctx
}
