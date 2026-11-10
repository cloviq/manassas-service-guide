import ExternalLink from './ExternalLink';
import { officialLinks } from './sources';
export default function CollectionGuidance() {
 return <><p className="journey-promo"><a href="/waste-collection.html">Waste collection, step by step</a><span>Learn the current rules, practice sorting, and find official next steps. Sources checked November 10, 2026.</span></p><details className="collection-guidance"><summary>Collection guidance · leaves, yard waste & trash</summary>
 <p>Seasonal loose-leaf areas are separate from regular yard waste and trash. Older map-layer collection windows do not state a year. The current City fall leaf page displays a 2026–27 schedule; confirm your collection period there.</p>
 <ExternalLink url={officialLinks.fall_leaf_guidance}>Current fall leaf guidance and lookup</ExternalLink>
 <p>The City’s yard-waste page says collection is on Mondays for City residents. Confirm preparation rules and current service guidance; this is not a polygon-based address assignment.</p>
 <ExternalLink url={officialLinks.yard_waste}>Yard waste information</ExternalLink>{' · '}
 <ExternalLink url={officialLinks.trash_guidance}>City page with Waste Wise schedule lookup</ExternalLink>
 <p>Use the official trash page’s Waste Wise lookup for your schedule. The bulk-area source has no collection-day field; it does not establish eligibility or a pickup day.</p></details></>;
}
