# Waste collection source review

Checked **2026-11-10 UTC**. These are public primary-source content checks, not address-lookup, request-submission, collection-outcome, or live-operational tests. Page modification dates were generally absent. No personal data was submitted. The learning page exposes these limits to readers.

## Sources used in the learning journey

| Source | Supported claims and exact evidence | Limits |
|---|---|---|
| [City service area](https://www.manassasva.gov/public_works/trash___recycling/about_us.php) | “services residents within the limits of the City of Manassas only.” Some Manassas/20110 addresses are not served. | ZIP/postal city is insufficient; individual address and building arrangements unverified. |
| [City trash and recycling landing page](https://www.manassasva.gov/public_works/trash___recycling/index.php) | “Use the City of Manassas Waste Wise widget below to view your collection schedule or learn about trash and recycling.” Report a Problem covers missed collection, cart exchange, bulk. Trashline 703-257-8252 during regular office hours. | Official entry point, not a direct submission or personal schedule result. Widget controls not tested. |
| [Trash](https://www.manassasva.gov/public_works/trash___recycling/trash.php) | Set carts out “prior to 6 a.m.” and retrieve after collection; no bags/raw trash on street. | Recovery timing conflicts with other pages. No operational promise. |
| [Recycling](https://www.manassasva.gov/public_works/trash___recycling/recycling.php) | Food/beverage metal cans and plastic containers, paper, flattened cardboard; rinse food containers. “Please no plastic bags!” “empty glass bottles and jars that are no longer recycled curbside.” Purple glass-only drop-off outside 8500 Public Works Drive, available any time. | Do not generalize all plastics as accepted, or all glass objects as suitable for drop-off. |
| [Yard waste](https://www.manassasva.gov/public_works/trash___recycling/yard_waste.php) | “Yard waste is collected on Mondays for all City residents.” Sunday-night setout. “No yard waste in plastic bags will be collected.” Paper bags or private containers marked Yard Waste; bundles ≤4 feet, branches ≤3 inches diameter, bundles ≤50 pounds. Current Fall guidance allows 10 bags and 10 bundles per pickup. | Numeric bag/bundle limit labeled Fall guidance, not universal/year-round. Holidays can change service. |
| [Fall leaf collection](https://www.manassasva.gov/public_works/trash___recycling/fall_leaf_collection.php) | October–January service; page image labels “2026-27 Fall Leaf Collection Schedule Bilingual.” Rake leaves to street/sidewalk edge the day before assigned period; no loose leaves in street outside season. | Separate seasonal vacuum route, not Monday bagged/bundled yard waste. Individual collection periods not verified. Older imported GIS windows lack year. |
| [Bulk waste and large items](https://www.manassasva.gov/public_works/trash___recycling/bulk_waste___large_items.php) | Call at least 48 hours before bulk day. “There is a limit of three (3) items per household per scheduled pickup.” Single-family Wednesday, townhouses Tuesday/Friday, potentially unlike trash day. TVs/computers separately scheduled. Chemicals, tires, mowers, large construction debris not normal bulk. | Specific item acceptance and appointment unverified. Confirm current day with City. |

## Conflict handling

- **Bulk notice:** The general trash page says 48 hours before trash day. The dedicated bulk page repeatedly says before bulk day and warns days differ. The learning journey follows the more specific page and visibly discloses this editorial resolution. It is not staff confirmation.
- **Missed pickup:** Trash/recycling pages publish a 2 p.m. same-day reporting cutoff, with later reports next day after 2 p.m. and Friday→Monday. Yard guidance and the City-linked [report form](https://form.jotform.com/201774768730160) say next business day and give crew hours 6 a.m.–6 p.m. The guide directs users to report promptly/confirm, without guaranteeing timing or declaring a miss solely because a cart is full at 2 p.m.
- **Leaf year:** The current dedicated leaf page labels 2026–27. Imported historical layer windows have no year. Preserve the old-layer warning without calling the current City page undated.

## Destinations and verification limits

- The existing [official leaf lookup](https://cityofmanassas.maps.arcgis.com/apps/instant/lookup/index.html?appid=39c57f5be60c4a2eb450601e0ae9e6f4) is still the exact target of the City's Leaf Collection Map link. It returned a Zone Lookup shell, not readable address results. The app now prioritizes the dedicated accessible City leaf page, which links the lookup.
- The City links Waste Wise to `https://webapp.recyclecoach.com/preview.html?country_cd=US&plugin_id=TWpBMk1RPT0%3D`. The shell was reached; interactive lookup was not tested. The guide uses the durable City landing page rather than claiming a direct working form.
- The City-linked missed-collection form requests name, exact address, email, phone, collection stream/day and optional photo. It was read only; the journey collects no such data and performs no submission.
- [Prince William County collection guidance](https://www.pwcva.gov/department/solid-waste-management/refuse-and-recycling-collection/) (page displays Revised 06/18/2025) says the County does not provide trash/recycling collection or bins; private hauler/HOA arrangements may apply. The journey simply flags separate jurisdictions rather than importing County or Manassas Park operational rules.
- New Resident Guide link failed during research. Historical noon-earliest setout and 24-hour removal claims were therefore not imported as current verified rules.

## Scope of change

This source review applies to the learning journey and its collection-guidance entry points. It does not certify every search result, imported GIS record, backend seed, or live operational state. Existing database records, migrations, indexes, and deployment are unchanged. Generic search links now say **Official service information**, because their URLs can be informational landing pages, not direct actions.
