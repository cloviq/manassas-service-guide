# Waste Collection Journey Implementation Plan

> **For agentic workers:** Use the native execution path for this delegated implementation; verify each task before completion.

**Goal:** Promote an evidence-backed, no-JavaScript waste collection learning journey without replacing the finder or map.

**Architecture:** Static HTML/CSS in Vite's public directory is served in development and copied unchanged to production output. Existing React surfaces link to it. Preserve the current backend and auth. Add an Nginx exact-route session guard for static HTML; never deploy.

**Tech Stack:** HTML, CSS, React/TypeScript, Vite, Vitest, Testing Library.

**Spec:** `docs/superpowers/specs/2026-11-10-waste-journey-design.md`

## Global constraints
- Preserve private access, current app behavior, and demo-only intake.
- No deployment, push, database migration, reindex, or invented operational facts.
- Visible citations, review date and limitations; six sections; no-JavaScript core and interaction.

## Review focus
- Incorrect jurisdiction: eligibility warning must be visible without action.
- Old bulk wording: use 48 hours before bulk day, not regular trash day.
- Disabled JavaScript: complete content and exercise remain in static HTML.
- Repeated exercise use: all explanations remain independently available with no stale state.
- Narrow screens/keyboard: wrapping layout, visible focus, native disclosure controls.

## Task 1: Build and cite the journey
- [x] Add failing `frontend/src/WasteJourney.test.tsx` tests for six sections, source-linked rules, native exercise, navigation and no scripts.
- [x] Run focused tests and confirm intended failures.
- [x] Add `frontend/public/waste-collection.html` and `frontend/public/waste-collection.css` with six sections and exact verified sources.
- [x] Add first-class navigation link in `App.tsx`; update `CollectionGuidance.tsx` and generic source action wording without claiming all records verified.
- [x] Run all frontend tests and production build.

## Task 2: Verify and document
- [x] Port onto user-confirmed public a5923cc baseline, preserving auth and source registry.
- [x] Add a failing session-boundary test, implement guard, and run isolated Nginx smoke with fake sessions.
- [x] Save source ledger and conflicts under `docs/`.
- [x] Run repository offline checks and report passed/failed/not-run distinctly.
- [x] Check local preview in Safari, including exercise expansion/collapse and main app navigation.
- [x] Review final diff, fix regressions, record remaining limitations, commit isolated branch. Do not push or deploy.
