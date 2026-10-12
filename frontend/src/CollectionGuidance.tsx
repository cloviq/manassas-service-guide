import ExternalLink from './ExternalLink';
import { officialLinks } from './sources';
export default function CollectionGuidance() {
 return <details className="collection-guidance"><summary>Collection guidance · leaves, yard waste & trash</summary>
 <p>Seasonal loose-leaf areas are separate from regular yard waste and trash. Published seasonal windows do not state a year; confirm the current season on the official map.</p>
 <ExternalLink url={officialLinks.leaf_lookup}>Official seasonal leaf lookup</ExternalLink>
 <p>The City’s yard-waste page says collection is on Mondays for City residents. Confirm preparation rules and current service guidance; this is not a polygon-based address assignment.</p>
 <ExternalLink url={officialLinks.yard_waste}>Yard waste information</ExternalLink>{' · '}
 <ExternalLink url={officialLinks.trash_guidance}>Trash schedule</ExternalLink>
 <p>Use the official trash page’s Waste Wise lookup for your schedule. The bulk-area source has no collection-day field; it does not establish eligibility or a pickup day.</p></details>;
}
