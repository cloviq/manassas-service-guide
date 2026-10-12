# Manassas Service Guide — frontend

React + TypeScript single-page app built with Vite, with MapLibre GL for the map.
It is an independent project, not a City website. After sign-in the first screen
is the service finder (search, resource type and five category controls); the
**Explore map** mode shows City GIS layers.

## Run

From the repository root, `make web` (or `npm run dev` here) starts the Vite dev
server on loopback; it prints its URL (http://127.0.0.1:5173 by default). Vite
proxies `/api` to `http://localhost:3001`, so run the Axum backend there
(`make api`). The app bundles `../catalog/sources.json`, which Vite is allowed to
read.

```sh
npm ci
npm run dev
npm test
npm run build      # TypeScript check, then dist/
npm run preview
```

In the Compose stack, nginx serves `dist/` at http://127.0.0.1:5174 with an SPA
fallback and proxies same-origin `/api` requests to the API container
(`nginx.conf`). The app embeds no sample records or fake responses; an
unavailable backend produces a retryable error.

## Structure

- `Root.tsx`: checks `/api/auth/me`, shows `Login.tsx` when signed out, and
  switches between the guide and, for admins, `AdminUsers.tsx` (**Manage users**:
  create accounts, change roles, reset passwords, delete). The server enforces
  sessions and roles; the client only chooses what to render.
- `App.tsx`: service finder, result details, street-segment lists and the demo
  request form (`DemoIntake.tsx`).
- `ExploreMap.tsx` / `MapLayers.tsx`: the map and its layer controls.
- `sources.ts`: reads map groups, layers, colors, field labels, notices and
  official links from the shared registry `catalog/sources.json`.
- `SourceDetails.tsx`: safe, labeled source-specific details.
- `CollectionGuidance.tsx`: official leaf lookup, yard-waste and trash schedule
  links shown with the Trash & recycling category.

## Backend contract

- `GET /api/services?q=&category=&type=&limit=&offset=` →
  `{items, total, limit, offset, search_mode, warning?}`. Modes: `hybrid`,
  `keyword`, `database`.
- Items: `id`, `name`, `description`, `category`, `provider_name`, `jurisdiction`,
  `official_url`, `source_url`, plus `record_type`, `source_key`, `details`,
  `links` and, for streets, `segment_count`.
- `GET /api/services/{id}` → item plus optional `instructions` and, for streets,
  `segments`.
- `GET /api/map/features` and `/api/map/features/{id}` → GeoJSON (see
  `backend/MAP-API.md`).
- `POST /api/tickets` → `{description, service_id?, contact_email?}`; response
  `{id, status: "new", demo: true}`.
- Category labels: Trash & recycling; Water & sewer; Electricity; Streets &
  sidewalks; Other services.

Provider and jurisdiction are displayed from API records, never inferred.
Keyword and database modes are labeled as lacking semantic matching; backend warnings and source links
are under About. Only absolute HTTP(S) links without URL credentials are
clickable, with `noopener noreferrer`.

## Map

- Find a service is the default mode. Switching modes hides, rather than resets,
  search state and demo drafts.
- MapLibre GL JS with an explicitly bundled module worker
  (`maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url`) and the standard
  OpenStreetMap raster tiles, with visible attribution. Tiles are requested
  interactively for the current view only; there is no bulk download, offline
  cache, service worker or prefetch. Follow the
  [OSM tile policy](https://operations.osmfoundation.org/policies/tiles/) and use a
  suitable provider before substantial traffic.
- Layer controls come from the registry's `map_groups` and `map_layers`: Streets,
  Snow routes, Collection (seasonal loose leaves, bulk waste area), Recreation
  (parks, walking trails, bike racks, bike routes), Watersheds, Parking districts
  and City places. Streets, all snow route types and seasonal leaves are on by
  default; the rest are opt-in. Groups with several children are collapsible and
  their parent checkbox selects all, clears all, or shows a native indeterminate /
  ARIA mixed state. Labels are 44px-high targets and support Tab/Space.
- Snow routes has one child per `Snow_Routes/0..3` source. Requests include the
  selected layers and, when snow is on, the selected `snow_types`; with every
  layer off no request is sent. The API filters before its feature cap, and the
  client also hides unchecked sources defensively.
- Colors come from source identity in the registry, not hashed classification
  strings. Only selected source types appear in the legend. Gold marks the
  selected feature, not a classification.
- Viewport requests are debounced 300 ms after `moveend`, abort obsolete requests
  and ignore stale results. Loading, empty, error/retry, disabled-layer and
  truncated states are explicit.
- Polygons render as fill plus outline, lines as lines and points as circles;
  geometry-type filters cover Multi* variants. Selecting a feature shows its
  allowlisted details, exact source-record link and separate service-information
  link; there is no GIS submission action. Click handling decodes the nested
  `details`/`links` JSON that MapLibre serializes.
- A route result's **Show on map** fetches that feature (or whole street group),
  draws it in a separate highlight source and fits the view to it. A truncated
  response is disclosed. No live plow status is inferred.
- Source dates are formatted in UTC; historical water-main breaks are labeled as
  not live; speed units are not inferred; the leaf windows' year is stated as
  unspecified.

## Demo requests and privacy

Demo submissions go to this project's backend, **not the City**. Do not enter
sensitive data or use the form for emergencies. Drafts remain in React memory with
no browser storage. A successful submission clears the draft; a failed request
keeps it for retry.

## Tests

`npm test` runs Vitest suites for sign-in and the admin page, search and category
selection, loading/empty/error/fallback states, details and source links,
unsafe-URL handling, keyboard search, stale-search protection, demo requests, map
layer controls and source details. Network responses are mocked only inside these
unit tests.

`scripts/` holds optional Playwright checks against a running frontend
(`FRONTEND_URL`, default http://127.0.0.1:5174):

```sh
python3 -m venv .venv && . .venv/bin/activate
python -m pip install playwright
python -m playwright install chromium
python3 scripts/check-mobile.py         # 320/390px layout, 44px targets, real API
python3 scripts/check-nested-map.py     # snow tri-state controls and server filtering
python3 scripts/check-expanded-map.py   # expanded layer controls and guidance links
python3 scripts/check-empty-search.py   # empty-result guidance with mocked search responses
```

They write screenshots and measurements to the ignored `evidence/` directory.
These scripts do not sign in, so against the current app they stop at the sign-in
page; they need a signed-in browser session to exercise the guide.

## Attribution

MapLibre/OSM attribution remains visible. See
[third-party notices](../THIRD-PARTY-NOTICES.md) and [data rights](../DATA-REUSE.md).
