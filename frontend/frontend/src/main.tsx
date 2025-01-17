import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { BrowserRouter } from "react-router";
import { SymphoniesProvider } from "./lib/SymphonyProvider.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <SymphoniesProvider>
        <App />
      </SymphoniesProvider>
    </BrowserRouter>
  </StrictMode>,
);
