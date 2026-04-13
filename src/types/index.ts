export interface ModelUsage {
  messages_5h: number
  messages_today: number
  messages_week: number
  output_tokens_5h: number
  output_tokens_today: number
  output_tokens_week: number
}

export interface UsageInfo {
  messages_today: number
  messages_5h_window: number
  sessions_today: number
  tool_calls_today: number
  messages_week: number
  output_tokens_5h: number
  output_tokens_today: number
  output_tokens_week: number
  last_rate_limit_at?: string
  is_rate_limited: boolean
  estimated_reset_at?: string
  session_reset_at?: string
  weekly_reset_at?: string
  subscription_type?: string
  rate_limit_tier?: string
  last_checked_at: string
  opus_usage: ModelUsage
  sonnet_usage: ModelUsage
}

export interface DailyActivity {
  date: string
  message_count: number
  session_count: number
  tool_call_count: number
}

export interface ProjectFolder {
  path: string
  name: string
}

export interface Account {
  id: string
  email: string
  label?: string
  account_uuid: string
  plan: 'pro' | 'free'
  added_at: string
  last_used_at?: string
  last_switched_at?: string
  is_active: boolean
  status: 'ok' | 'error' | 'expired'
  usage: UsageInfo
  projects: ProjectFolder[]
  selected_project?: number
  oauth_config?: Record<string, unknown>
}

export interface AccountUpdate {
  label?: string
  plan?: 'pro' | 'free'
  status?: 'ok' | 'error' | 'expired'
  usage?: UsageInfo
}

export interface AppConfig {
  language: 'vi' | 'en'
  theme: 'light' | 'dark' | 'system'
  vscode_path: string
  quota_warning_threshold: number
  auto_switch_on_empty: boolean
  launch_at_login: boolean
  claude_config_path: string
  claude_cli_path: string // empty = auto-detect
  backup_dir: string
  usage_refresh_interval: number // seconds, 0 = disabled
}

export interface SwitchResult {
  success: boolean
  from_email?: string
  to_email: string
  message: string
}

export interface QuotaSummary {
  messages_today: number
  messages_week: number
  sessions_today: number
  is_rate_limited: boolean
  estimated_reset_at?: string
  subscription_type?: string
}

export interface OAuthAccount {
  emailAddress: string
  accountUuid: string
}

export interface RealUsageData {
  success: boolean
  authenticated: boolean
  session_percent?: number
  weekly_all_percent?: number
  weekly_sonnet_percent?: number
  session_reset?: string
  weekly_reset?: string
  retry_after?: number
  error_message?: string
}

export interface TokenHealthResult {
  valid: boolean
  status: 'ok' | 'expired' | 'auth_error' | 'error' | 'network_error' | 'no_credentials' | 'invalid_credentials'
  organization_name?: string
  organization_role?: string
  error_message?: string
}

export interface CostUsageRecord {
  timestamp: string
  account_email: string
  account_label?: string
  account_id?: string
  model: string
  model_display: string
  input_tokens: number
  output_tokens: number
  cache_read_tokens: number
  cache_creation_tokens: number
  total_tokens: number
  estimated_cost_usd: number
  session_id: string
}
