import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

type BootstrapStatus = {
  appName: string;
  coreVersion: string;
  protocolVersion: string;
};

const browserPreviewStatus: BootstrapStatus = {
  appName: "M.I.O.",
  coreVersion: "browser preview",
  protocolVersion: "browser preview",
};

export function useBootstrapStatus() {
  const [status, setStatus] = useState<BootstrapStatus | null>(null);
  const [coreError, setCoreError] = useState(false);

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

  return {
    coreLabel: coreError ? "Core offline" : status ? "Core ready" : "Core connecting",
    coreReady: Boolean(status) && !coreError,
  };
}
