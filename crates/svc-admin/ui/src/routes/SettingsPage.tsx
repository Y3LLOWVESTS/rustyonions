// crates/svc-admin/ui/src/routes/SettingsPage.tsx
//
// WHAT: Settings screen for svc-admin.
// WHY:  Gives operators a place to adjust UI preferences (theme/language)
//       and inspect instance/auth metadata pulled from the backend.
//
// INTERACTS:
//   - api/adminClient.ts -> getUiConfig(), getMe()
//   - theme/ThemeProvider -> useTheme()
//   - i18n/useI18n -> useI18n()
//   - shared UI: LoadingSpinner, ErrorBanner

import React, { useEffect, useState } from 'react'
import { adminClient } from '../api/adminClient'
import type { MeResponse, UiConfigDto } from '../types/admin-api'
import { useTheme, type ThemeName } from '../theme/ThemeProvider'
import { useI18n } from '../i18n/useI18n'
import { LoadingSpinner } from '../components/shared/LoadingSpinner'
import { ErrorBanner } from '../components/shared/ErrorBanner'

type SettingsState = {
  loading: boolean
  error: string | null
  uiConfig: UiConfigDto | null
  me: MeResponse | null
}

export function SettingsPage() {
  const { theme, setTheme } = useTheme()
  const { locale, setLocale, t } = useI18n()

  const [state, setState] = useState<SettingsState>({
    loading: true,
    error: null,
    uiConfig: null,
    me: null
  })

  useEffect(() => {
    let cancelled = false

    async function load() {
      try {
        const [uiConfig, me] = await Promise.all([
          adminClient.getUiConfig(),
          adminClient.getMe()
        ])

        if (cancelled) return

        setState({
          loading: false,
          error: null,
          uiConfig,
          me
        })
      } catch (err) {
        if (cancelled) return
        const msg = err instanceof Error ? err.message : 'Failed to load settings'
        setState({
          loading: false,
          error: msg,
          uiConfig: null,
          me: null
        })
      }
    }

    load()

    return () => {
      cancelled = true
    }
  }, [])

  const { loading, error, uiConfig, me } = state

  // Derive theme and language options from backend config, with sensible fallbacks.
  const themeOptions: ThemeName[] =
    (uiConfig?.availableThemes as ThemeName[] | undefined)?.filter(
      (name) => !!name
    ) ?? (['light', 'dark'] as ThemeName[])

  const languageOptions: string[] =
    uiConfig?.availableLanguages && uiConfig.availableLanguages.length > 0
      ? uiConfig.availableLanguages
      : ['en-US', 'es-ES']

  return (
    <div className="svc-admin-page svc-admin-page-settings">
      <header className="svc-admin-page-header">
        <h1>{t('nav.settings')}</h1>
        <p>Configure your admin UI preferences and inspect instance metadata.</p>
      </header>

      {loading && <LoadingSpinner />}

      {!loading && error && <ErrorBanner message={error} />}

      {!loading && !error && (
        <>
          {/* UI preferences card */}
          <section className="svc-admin-card" style={{ marginBottom: '1.5rem' }}>
            <h2>UI preferences</h2>
            <p style={{ marginTop: '0.25rem', marginBottom: '1rem' }}>
              These settings apply to your browser only. Backend defaults still come from{' '}
              <code>/api/ui-config</code>.
            </p>

            {/* Theme selector */}
            <div style={{ marginBottom: '1.5rem' }}>
              <h3 style={{ marginBottom: '0.25rem' }}>Theme</h3>
              <p style={{ marginTop: 0, marginBottom: '0.75rem', fontSize: '0.9rem' }}>
                Choose between light and dark themes for the console.
              </p>
              <div role="radiogroup" aria-label="Theme">
                {themeOptions.map((name) => (
                  <label
                    key={name}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      marginRight: '1rem',
                      fontSize: '0.95rem'
                    }}
                  >
                    <input
                      type="radio"
                      name="theme"
                      value={name}
                      checked={theme === name}
                      onChange={() => setTheme(name)}
                    />
                    <span style={{ marginLeft: '0.5rem', textTransform: 'capitalize' }}>
                      {name}
                    </span>
                  </label>
                ))}
              </div>
            </div>

            {/* Language selector */}
            <div>
              <h3 style={{ marginBottom: '0.25rem' }}>Language</h3>
              <p style={{ marginTop: 0, marginBottom: '0.75rem', fontSize: '0.9rem' }}>
                Select the language for labels and navigation inside svc-admin.
              </p>

              <select
                value={locale}
                onChange={(event) => setLocale(event.target.value)}
                aria-label="Select language"
                style={{
                  padding: '0.4rem 0.6rem',
                  borderRadius: 6,
                  border: '1px solid var(--svc-admin-color-border-subtle)',
                  backgroundColor: 'var(--svc-admin-color-bg-elevated)',
                  color: 'var(--svc-admin-color-text)'
                }}
              >
                {languageOptions.map((code) => (
                  <option key={code} value={code}>
                    {code}
                  </option>
                ))}
              </select>
            </div>
          </section>

          {/* Instance & access metadata card */}
          <section className="svc-admin-card">
            <h2>Instance &amp; access</h2>
            <p style={{ marginTop: '0.25rem', marginBottom: '1rem', fontSize: '0.9rem' }}>
              Read-only mode, app playground flag, and your current identity as reported
              by the svc-admin backend.
            </p>

            <dl
              style={{
                display: 'grid',
                gridTemplateColumns: 'minmax(0, 1fr) minmax(0, 2fr)',
                rowGap: '0.5rem',
                columnGap: '1rem',
                margin: 0
              }}
            >
              {uiConfig && (
                <>
                  <dt style={{ fontWeight: 600 }}>Read-only UI</dt>
                  <dd style={{ margin: 0 }}>
                    {uiConfig.readOnly ? 'Enabled (no mutations allowed)' : 'Disabled'}
                  </dd>

                  <dt style={{ fontWeight: 600 }}>App playground</dt>
                  <dd style={{ margin: 0 }}>
                    {uiConfig.dev?.enableAppPlayground
                      ? 'Enabled for this instance'
                      : 'Disabled (hidden from UI)'}
                  </dd>

                  <dt style={{ fontWeight: 600 }}>Default theme</dt>
                  <dd style={{ margin: 0 }}>{uiConfig.defaultTheme}</dd>

                  <dt style={{ fontWeight: 600 }}>Default language</dt>
                  <dd style={{ margin: 0 }}>{uiConfig.defaultLanguage}</dd>
                </>
              )}

              {me && (
                <>
                  <dt style={{ fontWeight: 600 }}>Operator</dt>
                  <dd style={{ margin: 0 }}>
                    {me.displayName} ({me.subject})
                  </dd>

                  <dt style={{ fontWeight: 600 }}>Roles</dt>
                  <dd style={{ margin: 0 }}>
                    {me.roles && me.roles.length > 0 ? me.roles.join(', ') : 'none'}
                  </dd>

                  <dt style={{ fontWeight: 600 }}>Auth mode</dt>
                  <dd style={{ margin: 0 }}>{me.authMode}</dd>

                  {me.loginUrl && (
                    <>
                      <dt style={{ fontWeight: 600 }}>Login URL</dt>
                      <dd style={{ margin: 0 }}>
                        <a href={me.loginUrl}>{me.loginUrl}</a>
                      </dd>
                    </>
                  )}
                </>
              )}
            </dl>
          </section>
        </>
      )}
    </div>
  )
}
