import React from "react";
import ReactDOM from "react-dom/client";
import { loadCoverDir } from "./lib/artwork";
import { captureGlobalErrors } from "./lib/log";
import { loadSettings } from "./lib/settings";
import { AppThemeProvider } from "./themes/AppThemeProvider";
import AppLayout from "./app/AppLayout";

captureGlobalErrors();

// Settings load first so the very first render already has the saved theme. The folder of the
// cached covers is read with them, since a cover is drawn from the first render on.
Promise.all([loadSettings(), loadCoverDir()]).then(() =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <AppThemeProvider>
        <AppLayout />
      </AppThemeProvider>
    </React.StrictMode>,
  ),
);
