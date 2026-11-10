# Waste collection learning journey

## Intent and scope
The user selected waste collection as the civic teaching journey, bringing existing guidance forward and fixing and verifying sources. Keep the existing independent service finder, map, and demo-only intake. Do not deploy, publish, change access, migrate the database, or reindex.

## Approach
Use a static, six-section page at `/waste-collection.html`, promoted from the existing app navigation and collection guidance. This builds on the current React/Vite frontend without new dependencies. A React-only journey was considered but would not meet the no-JavaScript reading goal. A larger redesign of the finder was rejected to preserve existing workflows.

## Design
1. Confirm City service jurisdiction; postal address is not eligibility.
2. Check the official Waste Wise schedule and prepare carts before 6 a.m.
3. Sort materials and practice with an empty glass jar. Three native disclosure choices explain the evidence and work by keyboard without JavaScript.
4. Separate regular Monday yard waste from seasonal loose leaves; preserve year/geometry limitations.
5. Arrange bulk collection using dedicated current guidance, not the general trash-day assumption.
6. Take the next official step and inspect source provenance and conflicts.

The final baseline is public cloviq/manassas-service-guide at a5923cc, confirmed by the user. Preserve its login, roles and shared source registry. Serve the static HTML behind an exact Nginx route that validates the existing session; use a relative sign-in redirect and no-store. Content and the native interaction need no JavaScript after sign-in. Loopback Vite previews do not enforce this production guard.

Use the existing green/neutral palette, visible focus, 44px targets, flexible small-screen grids, clear heading order and a skip link. No address collection, simulated schedule lookup, live collection guarantees, analytics, or City submissions. External destinations say whether they are guidance, lookups, or reporting entry points. Check dates describe a point-in-time source review, not automatic monitoring.

## Verification and limits
Test page structure, source destinations, all exercise choices/repeated use, navigation integration, safe links and no script requirement. Run all frontend tests/build and repository offline suites. Attempt Safari visual/keyboard checks against loopback only. Live tailnet page currently fails to connect in Safari; no security bypass is permitted. Preserve and disclose baseline dependency/build warnings.
