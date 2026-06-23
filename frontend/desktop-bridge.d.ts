interface TokitaiDesktopBridgeResponse<T = unknown> {
  ok?: boolean;
  status?: number;
  data?: T;
  error?: string;
  message?: string;
}

interface TokitaiDesktopBridge {
  invoke<T = unknown>(command: string, payload?: Record<string, unknown>): Promise<TokitaiDesktopBridgeResponse<T>>;
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
  __TOKITAI_DESKTOP_BRIDGE__?: TokitaiDesktopBridge | null;
}
