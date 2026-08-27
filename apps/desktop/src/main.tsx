import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { UiPreferencesProvider } from "./uiPreferences";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("M.I.O. root element was not found.");
}

createRoot(root).render(
  <StrictMode>
    <UiPreferencesProvider>
      <App />
    </UiPreferencesProvider>
  </StrictMode>,
);
