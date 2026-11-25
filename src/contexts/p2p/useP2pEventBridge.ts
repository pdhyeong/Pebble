import { useEffect, type Dispatch } from "react";
import { subscribeAllEvents } from "../../adapters/tauri/events";
import { COMPLETED_TRANSFER_VISIBLE_MS, eventToActions } from "../../core/p2pEvents";
import type { P2pAction } from "../../core/p2pState";

export function useP2pEventBridge(dispatch: Dispatch<P2pAction>, onSubscribed: () => void) {
  useEffect(() => {
    let active = true;
    const timers = new Set<ReturnType<typeof setTimeout>>();

    const subscription = subscribeAllEvents((event) => {
      for (const action of eventToActions(event, Date.now())) dispatch(action);

      if (event.name === "transfer-completed") {
        const transferId = event.payload.transfer_id;
        const timer = setTimeout(() => {
          timers.delete(timer);
          dispatch({ type: "transfer/removed", transferId });
        }, COMPLETED_TRANSFER_VISIBLE_MS);
        timers.add(timer);
      }
    });

    subscription.ready.then(() => {
      if (active) onSubscribed();
    });

    return () => {
      active = false;
      subscription.stop();
      timers.forEach(clearTimeout);
    };
  }, [dispatch, onSubscribed]);
}
