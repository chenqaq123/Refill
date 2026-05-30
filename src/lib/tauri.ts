import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  DashboardState,
  Profile,
  ProviderInput,
  ProviderTestInput,
  ProviderTestResult,
  ProviderUpdateInput,
  ProviderValidation,
  SwitchProgress,
  SwitchResult,
  UsageSummary,
  UsageWindowRecord,
} from "./types";

export const api = {
  listProfiles: () => invoke<Profile[]>("list_profiles"),
  refreshProfiles: () => invoke<DashboardState>("refresh_profiles"),
  switchProfile: (profileId: string) => invoke<SwitchResult>("switch_profile", { profileId }),
  saveCurrentProfile: () => invoke<Profile>("save_current_profile"),
  createProvider: (input: ProviderInput) => invoke<Profile>("create_provider", { input }),
  updateProvider: (profileId: string, input: ProviderUpdateInput) =>
    invoke<Profile>("update_provider", { profileId, input }),
  deleteProvider: (profileId: string) => invoke<void>("delete_provider", { profileId }),
  validateProvider: (profileId: string) => invoke<ProviderValidation>("validate_provider", { profileId }),
  testProviderConnection: (input: ProviderTestInput) =>
    invoke<ProviderTestResult>("test_provider_connection", { input }),
  usageSummary: () => invoke<UsageSummary>("usage_summary"),
  accountUsageHistory: (profileId: string) =>
    invoke<UsageWindowRecord[]>("account_usage_history", { profileId }),
  readProxyLog: () => invoke<string[]>("read_proxy_log"),
  openLoginTerminal: () => invoke<void>("open_login_terminal"),
  getSettings: () => invoke<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) => invoke<AppSettings>("update_settings", { settings }),
  onSwitchProgress: (handler: (progress: SwitchProgress) => void) =>
    listen<SwitchProgress>("switch-progress", (event) => handler(event.payload)),
};
