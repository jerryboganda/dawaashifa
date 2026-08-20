---
activation: Glob
glob: "apps/**/*.{svelte,ts,js,css,astro}"
description: Svelte 5, TypeScript, Tailwind and RTL conventions for the console, rider PWA and marketing site.
---

# Frontend conventions

## Stack
- **SvelteKit 2 + Svelte 5** (runes), TypeScript strict, Tailwind CSS
- `apps/console` — ops console, SPA mode, desktop-first
- `apps/rider` — rider PWA, mobile-only, offline-tolerant
- `apps/web` — Astro marketing site, static
- `apps/shared` — generated API client and shared UI primitives

## TypeScript
- `strict: true`. **No `any`.** Use `unknown` and narrow with a type guard.
- No non-null assertions (`!`) on API data. Handle the null case.
- Types for API data come from `@dawaa/shared`. Never redeclare them.

## Svelte 5 runes — not the legacy API
```svelte
<script lang="ts">
  import type { Order } from '@dawaa/shared';
  let { order }: { order: Order } = $props();
  let expanded = $state(false);
  let itemCount = $derived(order.items.length);
</script>
```
Do not use `export let`, `$:` reactive statements, or the legacy store contract in new code.

## Every data view handles three states
Loading, empty, and error. All three, every time. A component that renders a server collection and only handles the success case is incomplete.

```svelte
{#if query.isLoading}
  <Skeleton rows={5} />
{:else if query.error}
  <ErrorState error={query.error} onRetry={query.refetch} />
{:else if query.data.length === 0}
  <EmptyState message={m.orders_empty()} />
{:else}
  {#each query.data as order (order.id)}<OrderRow {order} />{/each}
{/if}
```

## Money
Arrives as a string. Format only.
```ts
import { formatPkr } from '@dawaa/shared';
formatPkr(order.total); // "Rs 1,250.00"
```
Never `Number(order.total)`. Never sum line totals in the browser — the server sends the total.

## Internationalisation and RTL
- Three locales: `en`, `ur` (Urdu script, RTL), `ur-Latn` (Roman Urdu, LTR).
- **Never hardcode a user-facing string.** Every string goes through the message catalogue.
- Urdu is right-to-left. Use logical CSS properties throughout: `ms-4` not `ml-4`, `text-start` not `text-left`, `border-e` not `border-r`.
- Test every customer-facing screen with `dir="rtl"` before marking it done.
- Urdu text needs a font with proper Nastaliq or clean Naskh rendering. Use the configured font stack; do not fall back to a system default.

## Dates and numbers
- Display in `Asia/Karachi`. The API sends UTC ISO 8601.
- Phone numbers display local (`0300 1234567`), transmit E.164 (`+923001234567`).

## Styling
- Tailwind utility classes. No inline `style=`, no CSS-in-JS, no component-scoped CSS unless genuinely unavoidable.
- Design tokens live in `tailwind.config.ts`. Do not introduce arbitrary hex values in components.
- Touch targets on the rider PWA are minimum 44×44px. Riders use it one-handed, outdoors, in sunlight.

## Rider PWA specifics
- Assume the network will drop mid-task. Queue writes in IndexedDB, sync on reconnect, show sync state in the UI.
- Never block the UI on a network call. Optimistic update, reconcile after.
- Camera and GPS access must degrade gracefully when denied.

## Accessibility
- Semantic HTML first. `<button>` for actions, `<a>` for navigation.
- Every form control has an associated label.
- Keyboard navigable — the pharmacist review screen is used heavily and must be operable without a mouse.

## Performance
- Route-level code splitting is on by default. Do not defeat it with barrel imports of the whole app.
- Virtualise any list that can exceed 100 rows (inbox, order board, stock).
- The console runs on branch machines that are not fast. Budget accordingly.
