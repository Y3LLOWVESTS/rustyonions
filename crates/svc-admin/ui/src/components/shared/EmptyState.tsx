import React from 'react'

type Props = { message: string }

export function EmptyState({ message }: Props) {
  return (
    <div className="svc-admin-empty-state">
      {message}
    </div>
  )
}
