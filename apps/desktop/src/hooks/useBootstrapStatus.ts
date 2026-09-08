import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type BootstrapStatus = {
  appName: string;
  coreVersion: string;
  protocolVersion: string;
};

type CommandToolchainStartupStatus = {
  state: "checking" | "ready";
  elapsedMilliseconds: number | null;
};

const browserPreviewStatus: BootstrapStatus = {
  appName: "M.I.O.",
  coreVersion: "browser preview",
  protocolVersion: "browser preview",
};

export function useBootstrapStatus() {
  const [status, setStatus] = useState<BootstrapStatus | null>(null);
  const [coreError, setCoreError] = useState(false);
  const [toolchainStatus, setToolchainStatus] = useState<CommandToolchainStartupStatus>({
    state: "checking",
    elapsedMilliseconds: null,
  });

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      setStatus(browserPreviewStatus);
      return;
    }

    void invoke<BootstrapStatus>("bootstrap_status")
      .then(setStatus)
      .catch(() => {
        setCoreError(true);
      });
  }, []);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      setToolchainStatus({ state: "ready", elapsedMilliseconds: 0 });
      return;
    }

    let cancelled = false;
    let timeout: number | undefined;
    const refresh = async () => {
      try {
        const next = await invoke<CommandToolchainStartupStatus>(
          "desktop_command_toolchain_startup_status",
        );
        if (cancelled) return;
        setToolchainStatus(next);
        if (next.state !== "ready") {
          timeout = window.setTimeout(refresh, 250);
        }
      } catch {
        if (!cancelled) {
          timeout = window.setTimeout(refresh, 500);
        }
      }
    };
    void refresh();

    return () => {
      cancelled = true;
      if (timeout !== undefined) window.clearTimeout(timeout);
    };
  }, []);

  return {
    coreLabel: coreError ? "Core offline" : status ? "Core ready" : "Core connecting",
    coreReady: Boolean(status) && !coreError,
    toolchainReady: toolchainStatus.state === "ready",
    toolchainDiscoveryMilliseconds: toolchainStatus.elapsedMilliseconds,
  };
}
