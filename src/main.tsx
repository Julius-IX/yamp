import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app/App";
import { captureGlobalErrors } from "./lib/log";
import { loadSettings } from "./lib/settings";
import { AppThemeProvider } from "./themes/AppThemeProvider";

captureGlobalErrors();

// Settings load first so the very first render already has the saved theme.
loadSettings().then(() =>
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <AppThemeProvider>
        <App />
      </AppThemeProvider>
    </React.StrictMode>,
  ),
);
