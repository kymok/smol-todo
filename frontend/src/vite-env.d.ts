/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set to "1" in dev to show the debug title-bar background overlay. */
  readonly VITE_DEBUG_TITLEBAR?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
