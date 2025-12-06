// crates/svc-admin/ui/src/i18n/index.ts
//
// RO:WHAT — Minimal i18n provider for svc-admin (locale + t(key)).
// RO:WHY  — Let the backend drive the default locale via UiConfigDto
//           while keeping the client-side surface tiny and composable.
// RO:INTERACTS — api/adminClient.ts (getUiConfig),
//                public/locales/<locale>.json bundles,
//                LanguageSwitcher component.
// RO:INVARIANTS — Locale is a BCP-47-ish string ("en-US", "es-ES").
//                 Translation bundles are flat key→string maps.

import React, {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode
} from 'react'
import { adminClient } from '../api/adminClient'

type I18nContextValue = {
  locale: string
  t: (key: string) => string
  setLocale: (locale: string) => void
}

const DEFAULT_LOCALE = 'en-US'

const I18nContext = createContext<I18nContextValue | undefined>(undefined)

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

  // Fetch UI config once and adopt backend defaultLanguage as the starting locale.
  useEffect(() => {
    let cancelled = false

    adminClient
      .getUiConfig()
      .then((cfg) => {
        if (cancelled) return

        // Contract: UiConfigDto.defaultLanguage (camelCase)
        const backendLocale = cfg.defaultLanguage

        if (backendLocale && typeof backendLocale === 'string') {
          // Only override if we’re still on the hardcoded default.
          setLocale((current) =>
            current === DEFAULT_LOCALE ? backendLocale : current
          )
        }
      })
      .catch(() => {
        // Developer preview: if /api/ui-config fails, we stay on DEFAULT_LOCALE.
      })

    return () => {
      cancelled = true
    }
  }, [])

  const t = (key: string): string => {
    return messages[key] ?? key
  }

  const value: I18nContextValue = {
    locale,
    t,
    setLocale
  }

  // NOTE: This file is `.ts`, so we use React.createElement instead of JSX.
  return React.createElement(I18nContext.Provider, { value }, children)
}

export function useI18n(): I18nContextValue {
  const ctx = useContext(I18nContext)
  if (!ctx) {
    throw new Error('useI18n must be used within I18nProvider')
  }
  return ctx
}
