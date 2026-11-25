import { createContext, useContext, useReducer, type ReactNode } from "react";
import { hasActiveTransfer, p2pReducer } from "../core/p2pReducer";
import { initialP2pState, type P2pState } from "../core/p2pState";
import { useP2pActions } from "./p2p/useP2pActions";
import { useP2pEventBridge } from "./p2p/useP2pEventBridge";

type P2pActions = Omit<ReturnType<typeof useP2pActions>, "initialize">;

interface P2pContextValue extends P2pState, P2pActions {
  isDownloading: boolean;
  isUploading: boolean;
}

const P2pContext = createContext<P2pContextValue | null>(null);

export function P2pProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(p2pReducer, initialP2pState);
  const { initialize, ...actions } = useP2pActions(dispatch, state.discoveredPeers);

  useP2pEventBridge(dispatch, initialize);

  return (
    <P2pContext.Provider
      value={{
        ...state,
        isDownloading: actions.pendingDownloads > 0 || hasActiveTransfer(state, "download"),
        isUploading: actions.pendingUploads > 0 || hasActiveTransfer(state, "upload"),
        ...actions,
      }}
    >
      {children}
    </P2pContext.Provider>
  );
}

export function useP2pContext() {
  const context = useContext(P2pContext);
  if (!context) {
    throw new Error("useP2pContext must be used within a P2pProvider");
  }
  return context;
}
