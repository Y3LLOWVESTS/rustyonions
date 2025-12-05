// crates/svc-admin/ui/src/i18n/index.ts

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  ReactNode
} from 'react'
import { adminClient } from '../api/adminClient'

type I18nContextValue = {
  locale: string
  t: (key: string) => string
  setLocale: (locale: string) => void
}

const I18nContext = createContext<I18nContextValue | undefined>(undefined)

const DEFAULT_LOCALE = 'en-US'

type ProviderProps = {
  children: ReactNode
}

export const I18nProvider = ({ children }: ProviderProps) => {
  const [locale, setLocale] = useState(DEFAULT_LOCALE)
  const [messages, setMessages] = useState<Record<string, string>>({})

  // Load translation bundle whenever the locale changes.
  useEffect(() => {
    let cancelled = false

    fetch(`/locales/${locale}.json`)
      .then((rsp) => rsp.json())
      .then((data: Record<string, string>) => {
        if (cancelled) return
        setMessages(data)
      })
      .catch(() => {
        if (cancelled) return
        // If we fail to load the locale, fall back to an empty map and let keys show.
        setMessages({})
      })

    return () => {
      cancelled = true
    }
  }, [locale])

  // Fetch UI config once and adopt backend default_locale as the starting locale.
  useEffect(() => {
    let cancelled = false

    adminClient
      .getUiConfig()
      .then((cfg) => {
        if (cancelled) return

        const backendLocale = (cfg as any).default_locale ?? (cfg as any).default_language

        if (backendLocale && typeof backendLocale === 'string') {
          setLocale((current) => (current === DEFAULT_LOCALE ? backendLocale : current))
        }
      })
      .catch(() => {
        // Developer preview: if /api/ui-config fails, we stay on the local default locale.
      })

    return () => {
      cancelled = true
    }
  }, [])

  const t = (key: string) => messages[key] ?? key

  return React.createElement(
    I18nContext.Provider,
    { value: { locale, t, setLocale } },
    children
  )
}

export function useI18n() {
  const ctx = useContext(I18nContext)
  if (!ctx) {
    throw new Error('useI18n must be used within I18nProvider')
  }
  return ctx
}
