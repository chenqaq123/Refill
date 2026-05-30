export type ProfileKind = "official" | "api";

export type UsageWindow = {
  label: string;
  remainingPercent: number;
  resetsAt?: number | null;
  resetText: string;
  isEstimatedRecovered: boolean;
};

export type UsageSnapshot = {
  timestamp: string;
  primary?: UsageWindow | null;
  secondary?: UsageWindow | null;
  hasEstimatedRecovery: boolean;
};

export type WireApi = "responses" | "chat";

export type ApiProvider = {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  providerId: string;
  wireApi: WireApi;
  keyStatus: "exists" | "missing";
  createdAt: string;
};

export type ProfileDiagnostics = {
  profilePath: string;
  codexHomePath: string;
  sessionsShared: boolean;
  sessionIndexShared: boolean;
  desktopStateShared: boolean;
  workspaceShared: boolean;
  configExists: boolean;
  authExists: boolean;
  keychainReady: boolean;
};

export type Profile = {
  id: string;
  kind: ProfileKind;
  title: string;
  subtitle: string;
  primaryPill: string;
  isActive: boolean;
  isReady: boolean;
  usage?: UsageSnapshot | null;
  provider?: ApiProvider | null;
  diagnostics: ProfileDiagnostics;
};

export type DashboardState = {
  profiles: Profile[];
  unmanagedCurrent?: Profile | null;
  activeLabel: string;
  profileRoot: string;
  codexHome: string;
  sharedHistoryRoot: string;
  lastSyncedAt: string;
};

export type ProviderInput = {
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  wireApi: WireApi;
};

export type ProviderUpdateInput = {
  name: string;
  baseUrl: string;
  model: string;
  apiKey?: string | null;
  wireApi: WireApi;
};

export type SwitchStage =
  | "quitting_codex"
  | "syncing_current"
  | "preparing_target"
  | "sharing_history"
  | "linking_codex_home"
  | "launching_codex"
  | "done"
  | "failed";

export type SwitchProgress = {
  profileId: string;
  stage: SwitchStage;
  message: string;
  percent?: number | null;
};

export type SwitchResult = {
  profileId: string;
  launched: boolean;
};

export type ProviderValidation = {
  ok: boolean;
  message: string;
};

export type ProviderTestInput = {
  baseUrl: string;
  model: string;
  wireApi: WireApi;
  apiKey?: string | null;
  profileId?: string | null;
};

export type ProviderTestResult = {
  ok: boolean;
  status: number;
  latencyMs: number;
  message: string;
  suggestChat: boolean;
};

export type ModelUsage = {
  model: string;
  requests: number;
  inputTokens: number;
  outputTokens: number;
};

export type ProviderUsage = {
  providerId: string;
  name: string;
  requests: number;
  inputTokens: number;
  outputTokens: number;
  models: ModelUsage[];
};

export type UsageSummary = {
  totalRequests: number;
  totalInputTokens: number;
  totalOutputTokens: number;
  providers: ProviderUsage[];
};

export type AppSettings = {
  refreshIntervalSeconds: number;
  shareHistory: boolean;
  codexAppName: string;
};
