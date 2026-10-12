# Third-party software and external resources

The root MIT license covers project-owned code, documentation and example fixtures only.
License texts for installed React, React DOM and MapLibre runtime versions are
retained under `licenses/`. They keep their upstream copyright and component
notices. No dependency source, vendored library, model weights, GIS data or map tiles are
bundled in this source repository. Dependencies retain their own licenses and
notices when installed. Lockfiles preserve dependency identity; licenses of all
transitive dependencies must accompany any redistributed binary, image or bundle.

- React / React DOM: MIT; https://github.com/facebook/react/blob/main/LICENSE
- Vite: MIT; https://github.com/vitejs/vite/blob/main/LICENSE
- MapLibre GL JS: BSD-3-Clause with additional component notices, including earlier
  Mapbox code; https://github.com/maplibre/maplibre-gl-js/blob/main/LICENSE.txt
- Rust dependencies are listed in `backend/Cargo.toml` / `Cargo.lock`; their crate
  distributions carry the applicable license files. OpenSearch/PostgreSQL and base
  container images are separately acquired, not relicensed by this repository.
- FastEmbed / ONNX Runtime and Python dependencies retain their package notices.
  BGE and Qdrant ONNX model artifacts are separately downloaded; consult their
  artifact licenses and model cards before redistribution. Model weights are not
  included, and no license is inferred from a local download manifest.
- OpenStreetMap attribution remains visible in `frontend/src/ExploreMap.tsx`:
  © OpenStreetMap contributors. Map/data terms:
  https://www.openstreetmap.org/copyright ; tile service policy:
  https://operations.osmfoundation.org/policies/tiles/
- City of Manassas and ArcGIS source URLs are optional data references, not MIT
  data grants or endorsements. See [DATA-REUSE.md](DATA-REUSE.md).

For a built frontend, include the license texts from the exact installed runtime
packages and bundled transitive dependencies. For a backend/container distribution,
retain crate/package/base-image notices. This source notice is not a complete
binary-distribution notice inventory.
