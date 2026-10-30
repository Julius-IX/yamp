import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app/App";
import { captureGlobalErrors } from "./lib/log";

captureGlobalErrors();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
