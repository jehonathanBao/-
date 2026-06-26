import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App.jsx";
import PageErrorBoundary from "./components/PageErrorBoundary.jsx";
import PageShellSkeleton from "./components/PageShellSkeleton.jsx";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <BrowserRouter>
      <PageErrorBoundary>
        <React.Suspense fallback={<PageShellSkeleton />}>
          <App />
        </React.Suspense>
      </PageErrorBoundary>
    </BrowserRouter>
  </React.StrictMode>,
);
