import { useSyncExternalStore } from "react";

export interface RouteAddress { readonly hash: string; readonly generation: symbol }
let address: RouteAddress = { hash: window.location.hash, generation: Symbol("address") };
const listeners = new Set<() => void>();

/** Check the real address at admission and delivery, before React effects run. */
export function getRouteAddress(): RouteAddress {
  if (address.hash !== window.location.hash) address = { hash: window.location.hash, generation: Symbol("address") };
  return address;
}
window.addEventListener("hashchange", () => {
  // Each navigation invalidates previous work, including an A → B → A trip.
  address = { hash: window.location.hash, generation: Symbol("address") };
  for (const listener of [...listeners]) listener();
});
export function subscribeRouteAddress(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export function useRouteAddress(): RouteAddress {
  return useSyncExternalStore(subscribeRouteAddress, getRouteAddress);
}
