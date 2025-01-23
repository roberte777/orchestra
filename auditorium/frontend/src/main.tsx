import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { SymphoniesProvider } from "./lib/SymphonyProvider.tsx";
import App from "./index.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <SymphoniesProvider>
      <App />
    </SymphoniesProvider>
  </StrictMode>,
);
