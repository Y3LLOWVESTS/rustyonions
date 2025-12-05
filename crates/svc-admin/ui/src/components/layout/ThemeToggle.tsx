import React from 'react'
import { useTheme } from '../../theme/ThemeProvider'

export function ThemeToggle() {
  const { theme, setTheme } = useTheme()

  const isLight = theme === 'light'
  const nextTheme = isLight ? 'dark' : 'light'
  const label = isLight ? 'Switch to dark theme' : 'Switch to light theme'
  const icon = isLight ? '☾' : '☀︎'

  const handleClick = () => {
    setTheme(nextTheme)
  }

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className="svc-admin-theme-toggle"
      onClick={handleClick}
    >
      {icon}
    </button>
  )
}
