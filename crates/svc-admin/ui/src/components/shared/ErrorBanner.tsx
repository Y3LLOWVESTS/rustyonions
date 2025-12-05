import React from 'react'

type Props = { message: string }

export function ErrorBanner({ message }: Props) {
  return (
    <div className="svc-admin-error-banner">
      {message}
    </div>
  )
}
