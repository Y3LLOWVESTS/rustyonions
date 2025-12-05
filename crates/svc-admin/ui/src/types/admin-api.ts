export type UiConfigDto = {
  default_theme: string
  available_themes: string[]
  default_language: string
  available_languages: string[]
  read_only: boolean
}

export type MeResponse = {
  subject: string
  display_name: string
  roles: string[]
  auth_mode: string
  login_url: string | null
}

export type NodeSummary = {
  id: string
  display_name: string
  profile?: string | null
}

export type PlaneStatus = {
  name: string
  health: string
  ready: boolean
  restart_count: number
}

export type AdminStatusView = {
  id: string
  display_name: string
  profile?: string | null
  version?: string | null
  planes: PlaneStatus[]
}
