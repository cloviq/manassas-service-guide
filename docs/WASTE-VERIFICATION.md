# Waste journey verification

Date: 2026-11-10 UTC. Final branch: `feat/waste-learning-journey-current`, based on user-confirmed public `cloviq/manassas-service-guide` commit `a5923cc6c224cab8ba34385b4e44e904f773b4a3`. Work is in an isolated Mac clone. No push, deployment, access-setting change, database migration, ticket submission, or reindex was performed.

The first prototype was built against older private commit `0a94de3` and preserved separately as `80e7e7e`. The final branch ports the scoped changes onto the newer public baseline, preserving its login/roles, source registry, backend and newer service catalog. The older branch is not the final deliverable.

## Passed on the final baseline

- `make test PYTHON=python3.12` (command-local PATH includes `~/.cargo/bin`): Rust **39 passed, 0 failed, 12 ignored**; frontend **55 passed**; Python catalog **5**, workflow **5**, container **7** passed. Ignored live tests are not integration evidence.
- `cargo build --locked`, `cargo fmt --check`, and `cargo clippy --locked --offline --all-targets -- -D warnings`: passed. The first offline attempt lacked cached argon2; fetching the locked crates resolved it without dependency changes.
- `npm run build`: TypeScript and Vite passed. Generated journey HTML/CSS match their public source files byte-for-byte.
- `git diff --check`: passed.
- Test-first checks: four missing journey/navigation tests failed before implementation; a missing no-JavaScript landing-link test and a missing session-boundary contract test also failed before their changes. All now pass.
- Independent review and primary-source checks: all seven City pages matched the substantive claims. Review covered both the first implementation and its port, including auth, registry and Nginx changes.

## Session-boundary runtime test

`frontend/scripts/check-waste-session.py` is an optional reproducible smoke test, separate from the offline suite. It runs official `nginx:1.28-alpine` (observed Nginx 1.28.3) in a network-disabled, automatically removed container with read-only test config and built frontend mounts. A fake in-container session endpoint replaces only the upstream address. No real account, password, database, or production server is used.

Verified:
- Missing/invalid session → 302 with actual relative `Location: /`, including simulated upstream HTTPS.
- Valid fake session → 200 and the expected guide; `Cache-Control: private, no-store`.
- Forbidden session → 403; unavailable auth upstream → fail-closed 500 (expected Nginx error log).
- Direct internal auth route → 404.
- HEAD, query string, duplicate-slash, dot-segment and percent-encoded path variants retain protection.

Review caught an initial Nginx error-page form that returned status 302 without a Location header. The runtime assertion reproduced it. A named handler with explicit `return 302 /` and `absolute_redirect off` fixed it; all smoke cases then passed. This test validates Nginx behavior, not a real backend session integration.

## Native Safari checks

Production preview at `http://127.0.0.1:5186`: guide rendered, six sections and links appeared in accessibility tree, section anchors worked, glass-choice expansion/collapse and Space-key toggling worked, feedback was readable, and return-to-finder navigation worked before the final baseline port. The guide assets carried into the final port unchanged except the explicit sign-in copy. Reloading the final app root in Safari showed the existing invited-account sign-in screen, with no service content exposed while signed out. Increased-zoom narrow layout reflowed to one column without visible clipping in inspected views; zoom was restored.

A clipped disclosure focus ring was reproduced, fixed with an inset outline, and visually rechecked. Exact 320px/390px viewport measurements, automated browser accessibility scanning and a full screen-reader audit were not run. Native Safari had no browser-automation/responsive-mode surface available; zoomed-layout inspection is not an exact device-width check.

**Preview distinction:** Vite dev/preview is loopback-only and does not enforce the Nginx session guard. Do not expose it as the deployed app. Deployed guide requests need the existing API-backed session. Sign-in requires JavaScript; after sign-in the guide and its native exercise do not. The browser's global JavaScript setting was not changed; static content and native controls were tested directly.

## Not run / not claimed

- No live backend search/map/ticket integration, downloaded model tests, address-specific Waste Wise results, official report submission, or collection outcomes.
- The live tailnet URL failed in Safari with “server unexpectedly dropped the connection.” No security warning, authentication gate, or network restriction was bypassed. Local checks do not establish deployment success.
- No dependency/security upgrade was bundled into this task. Final baseline npm audit reports **2 moderate and 1 high** advisories (`@vitest/mocker`, `vitest`, `vite`); package inputs unchanged.
- Existing Vite >500kB bundle and npm `http-proxy` environment warnings remain. The static guide does not load the map bundle.

## Delivery boundary

The source review applies to the learning page and updated collection-guidance entry points, not every imported/API record. The new Nginx route preserves the sign-in boundary rather than changing who can access the site. This local branch is ready for review, not published or deployed.
