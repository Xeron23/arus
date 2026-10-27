import { isTauri } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { version } from "../package.json";

export let appVersion = version;
export async function loadAppVersion(): Promise<void> {
  if (isTauri()) appVersion = await getVersion();
}
