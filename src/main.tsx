import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ThemeProvider } from "@/hooks";
import { Bar } from "@/bar";
import { Dashboard } from "@/dashboard";
import { ScreenAreaPicker } from "@/ocr";
import { DrawCanvas, DrawToolbar } from "@/draw";
import "./global.css";

// Every window loads the same page; its label decides what it shows
const label = getCurrentWindow().label;

const view =
  label === "ocr-overlay" ? (
    <ScreenAreaPicker />
  ) : label === "draw-canvas" ? (
    <DrawCanvas />
  ) : label === "draw-toolbar" ? (
    <ThemeProvider>
      <DrawToolbar />
    </ThemeProvider>
  ) : (
    <ThemeProvider>{label === "dashboard" ? <Dashboard /> : <Bar />}</ThemeProvider>
  );

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<React.StrictMode>{view}</React.StrictMode>);
