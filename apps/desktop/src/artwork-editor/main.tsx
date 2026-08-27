import { createRoot } from "react-dom/client";

import { ArtworkEditorApp } from "./ArtworkEditorApp";
import "./artwork-editor.css";

const root = document.getElementById("artwork-editor-root");

if (!root) {
  throw new Error("M.I.O. artwork editor root was not found.");
}

createRoot(root).render(<ArtworkEditorApp />);
