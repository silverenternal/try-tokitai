interface AtlasDesktopBridgeResponse<T = unknown> {
  ok?: boolean;
  status?: number;
  data?: T;
  error?: string;
  message?: string;
}

interface AtlasDesktopBridge {
  invoke<T = unknown>(command: string, payload?: Record<string, unknown>): Promise<AtlasDesktopBridgeResponse<T>>;
  openStream?(
    command: string,
    payload?: Record<string, unknown>,
  ): Promise<Response>;
}

/**
 * Command examples:
 * - `bootstrap.load`
 * - `chat.send`
 * - `chat.stream`
 * - `workspace.pick`
 * - `workspace.file.open`
 * - `workspace.file.save`
 * - `git.action`
 * - `terminals.create`
 */

interface Window {
  __ATLAS_DESKTOP_BRIDGE__?: AtlasDesktopBridge | null;
  __ATLAS_REQUEST_CLOSE__?: () => void;
  __ATLAS_HOST__?: {
    restoreState?: { client?: Record<string, unknown> } | null;
    [key: string]: unknown;
  };
}
