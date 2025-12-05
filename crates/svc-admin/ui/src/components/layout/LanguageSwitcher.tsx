import React from 'react'
import { useI18n } from '../../i18n'

export function LanguageSwitcher() {
  const { locale, setLocale } = useI18n()

  const handleChange = (event: React.ChangeEvent<HTMLSelectElement>) => {
    setLocale(event.target.value)
  }

  return (
    <select
      className="svc-admin-language-switcher"
      value={locale}
      onChange={handleChange}
      aria-label="Select language"
    >
      <option value="en-US">EN</option>
      <option value="es-ES">ES</option>
    </select>
  )
}
