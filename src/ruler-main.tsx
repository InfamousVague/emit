import React from "react";
import ReactDOM from "react-dom/client";
import { RulerApp } from "./components/Ruler/RulerApp";

ReactDOM.createRoot(document.getElementById("ruler-root")!).render(
  <React.StrictMode>
    <RulerApp />
  </React.StrictMode>,
);
