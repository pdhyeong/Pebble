import { listen, TauriEvent, type UnlistenFn } from "@tauri-apps/api/event";
import { EVENT_NAMES, type ContractEvent, type EventName, type EventPayloads } from "../../core/contract";

export type Unsubscribe = () => void;

export interface Subscription {
  ready: Promise<void>;
  stop: Unsubscribe;
}

function toUnsubscribe(pending: Promise<UnlistenFn>): Unsubscribe {
  return () => {
    pending.then((unlisten) => unlisten());
  };
}

export function onEvent<K extends EventName>(
  name: K,
  handler: (payload: EventPayloads[K]) => void,
): Unsubscribe {
  return toUnsubscribe(listen<EventPayloads[K]>(name, (event) => handler(event.payload)));
}

export function subscribeAllEvents(handler: (event: ContractEvent) => void): Subscription {
  const pending = EVENT_NAMES.map((name) =>
    listen<EventPayloads[typeof name]>(name, (event) => {
      handler({ name, payload: event.payload } as ContractEvent);
    }),
  );
  return {
    ready: Promise.all(pending).then(() => undefined),
    stop: () => pending.forEach((p) => toUnsubscribe(p)()),
  };
}

export function onFileDrop(handler: (paths: string[]) => void): Unsubscribe {
  return toUnsubscribe(
    listen<{ paths: string[] }>(TauriEvent.DRAG_DROP, (event) => handler(event.payload.paths)),
  );
}
