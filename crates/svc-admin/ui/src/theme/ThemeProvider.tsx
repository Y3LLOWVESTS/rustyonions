import React, { createContext, useContext, useState, useEffect } from 'react'
import { themes } from './themes'
import { adminClient } from '../api/adminClient'

type Theme = keyof typeof themes

type ThemeContextValue = {
  theme: Theme
  setTheme: (theme: Theme) => void
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined)

const DEFAULT_THEME: Theme = 'light'

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [theme, setTheme] = useState<Theme>(DEFAULT_THEME)

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

        const candidate = cfg.default_theme as Theme | undefined

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

export function useTheme() {
  const ctx = useContext(ThemeContext)
  if (!ctx) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return ctx
}
