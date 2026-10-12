-- Additive: snow service tasks. Snow-route geometry is map data, not a service a
-- resident acts on. Text uses only facts stated on the cited official pages
-- Confirm current details at the source.
INSERT INTO providers (id,name,jurisdiction,official_url) VALUES
('10000000-0000-4000-8000-000000000002','City of Manassas Street Department (Snow Removal)','City of Manassas, Virginia','https://www.manassasva.gov/public_works/street_department/snow_removal/index.php')
ON CONFLICT (id) DO NOTHING;

INSERT INTO services (id,provider_id,name,description,category,official_url,source_url,instructions)
SELECT id::uuid,'10000000-0000-4000-8000-000000000002'::uuid,name,description,'Streets & sidewalks',official_url,source_url,instructions
FROM (VALUES
('20000000-0000-4000-8000-000000000007','Report a snow or unplowed street concern',
 'Find how to report a snow-related issue, including plowing on City streets, to the Street Department.',
 'https://www.manassasva.gov/public_works/street_department/snow_removal/index.php',
 'https://www.manassasva.gov/public_works/street_department/snow_removal/plowing.php',
 'The City asks residents to call the Snowline at (703) 257-8353 to report snow-related issues or concerns. Crews plow primary routes first, then secondary routes, then the 12 residential sections. For a privately maintained street, contact your homeowners association. This guide cannot submit requests to the City.'),
('20000000-0000-4000-8000-000000000008','Check snow emergency routes and parking rules',
 'See which streets are Snow Emergency Routes and when parked vehicles must be moved during snow.',
 'https://www.manassasva.gov/snowmap',
 'https://www.manassasva.gov/public_works/street_department/snow_removal/emergency_snow_routes.php',
 'If a Snow Emergency is declared, move your vehicle off the designated Snow Emergency Routes or it will be towed. City Code also prohibits parking on a snow emergency route once snow and ice reach four inches or more. Use the City snow route map to look up a street, and confirm current rules on the official page.'),
('20000000-0000-4000-8000-000000000009','Clear your sidewalk after snow',
 'Check the City rule for how quickly snow must be cleared from sidewalks next to your home.',
 'https://www.manassasva.gov/public_works/street_department/snow_removal/snow_and_the_law.php',
 'https://www.manassasva.gov/public_works/street_department/snow_removal/snow_and_the_law.php',
 'City Code requires the occupant to clear snow from abutting sidewalks within 24 hours after a snowfall of six inches or less, and within 48 hours after more than six inches. See the official page for the exact requirements.')
) AS tasks(id,name,description,official_url,source_url,instructions)
ON CONFLICT (id) DO NOTHING;
