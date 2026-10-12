# Data rights and local acquisition

The MIT license covers project-authored code, documentation and example fixtures.
It does not relicense municipal/GIS data, map tiles, third-party software or models.
Downloaded GIS files are not committed: the collectors fetch them from the City's
public ArcGIS services into ignored `catalog/` files when you run `make up` or
`make data-fetch`. The service tasks seeded by the database migrations summarize
facts stated on the cited official City pages and link to them; confirm current
details there.

The City of Manassas GIS hub is https://gis-cityofmanassas.hub.arcgis.com and its
source FAQ is https://gis-cityofmanassas.hub.arcgis.com/pages/faqs. Each ArcGIS
service/item may have separate terms, copyright and attribution. Public query
access alone does not establish a redistribution license. Before fetching, review
the applicable publisher and item terms; before redistributing, obtain sufficient
rights and record source URL, item ID, retrieval date, license/terms version,
copyright text, attribution and any restrictions. Unclear rights remain a blocker
for redistributing downloaded data, even when code is MIT licensed.

The collectors save source metadata, query URLs, object IDs, hashes and
retrieval manifests locally. Raw fields can include staff/editor identities and
work-order metadata. Keep them local; inspect and minimize any derived dataset
before sharing it. UI field allowlists are not a substitute for auditing the stored
raw download. Do not copy `catalog/raw`, manifests, snapshots, tickets or database
volumes into a public repository or build image.

No schedule, jurisdiction, provider, eligibility or live status is inferred from
geometry. Historical water-main breaks are not live outage alerts; seasonal leaf
windows may omit a year. Confirm current instructions at official source links.

MapLibre requests OpenStreetMap tiles interactively. Retain visible attribution
and comply with https://www.openstreetmap.org/copyright and
https://operations.osmfoundation.org/policies/tiles/. This repository includes no
tiles, bulk download, offline tile cache or OSM dataset. Use an appropriate provider
and terms before operating a high-traffic map.
