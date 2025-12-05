import React, { createContext, useContext, useState, useEffect } from 'react'

type I18nContextValue = {
  locale: string
  t: (key: string) => string
  setLocale: (locale: string) => void
}

const I18nContext = createContext<I18nContextValue | undefined>(undefined)

export const I18nProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [locale, setLocale] = useState('en-US')
  const [messages, setMessages] = useState<Record<string, string>>({})

  useEffect(() => {
    fetch(`/locales/${locale}.json`)
      .then((rsp) => rsp.json())
      .then((data) => setMessages(data))
      .catch(() => setMessages({}))
  }, [locale])

  const t = (key: string) => messages[key] ?? key

  return (
    <I18nContext.Provider value={{ locale, t, setLocale }}>
      {children}
    </I18nContext.Provider>
  )
}

export function useI18n() {
  const ctx = useContext(I18nContext)
  if (!ctx) throw new Error('useI18n must be used within I18nProvider')
  return ctx
}
