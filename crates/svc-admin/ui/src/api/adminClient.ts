import type { UiConfigDto, MeResponse, NodeSummary, AdminStatusView } from '../types/admin-api'

const base = ''

async function getJson<T>(path: string): Promise<T> {
  const rsp = await fetch(base + path)
  if (!rsp.ok) {
    throw new Error(`Request failed: ${rsp.status}`)
  }
  return rsp.json() as Promise<T>
}

export const adminClient = {
  getUiConfig: () => getJson<UiConfigDto>('/api/ui-config'),
  getMe: () => getJson<MeResponse>('/api/me'),
  getNodes: () => getJson<NodeSummary[]>('/api/nodes'),
  getNodeStatus: (id: string) => getJson<AdminStatusView>(`/api/nodes/${id}/status`)
}
