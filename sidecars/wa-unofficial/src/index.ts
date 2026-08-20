/**
 * Baileys WhatsApp Transport Sidecar (Doc 03 §4, §5)
 * Thin transport shim with Postgres auth persistence and NATS bridge.
 * Invariant I-10: No business logic lives in this sidecar.
 */

console.log("Shifa Baileys WhatsApp Transport Sidecar initializing...");
