import React from "react";
import ReactDOM from "react-dom/client";
import { captureGlobalErrors } from "./lib/log";
import { loadSettings } from "./lib/settings";
import { AppThemeProvider } from "./themes/AppThemeProvider";
import AppLayout from "./app/AppLayout";

captureGlobalErrors();

// Settings load first so the very first render already has the saved theme.
loadSettings().then(() =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <AppThemeProvider>
        <AppLayout />
      </AppThemeProvider>
    </React.StrictMode>,
  ),
);
