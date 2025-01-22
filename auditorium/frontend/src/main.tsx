import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { SymphoniesProvider } from "./lib/SymphonyProvider.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <SymphoniesProvider>
      <App />
    </SymphoniesProvider>
  </StrictMode>,
);
