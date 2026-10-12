-- Additive: service tasks for the Electricity, Water & sewer, Streets & sidewalks
-- and Other services categories. Text uses only facts stated on the cited official
-- pages; confirm current details at the source.
INSERT INTO providers (id,name,jurisdiction,official_url) VALUES
('10000000-0000-4000-8000-000000000003','City of Manassas Electric Department','City of Manassas, Virginia','https://www.manassasva.gov/electric_water___sewer/index.php'),
('10000000-0000-4000-8000-000000000004','City of Manassas Water & Sewer','City of Manassas, Virginia','https://www.manassasva.gov/city_departments/water___sewer/sewer/index.php'),
('10000000-0000-4000-8000-000000000005','City of Manassas Utility Customer Services','City of Manassas, Virginia','https://www.manassasva.gov/electric_water___sewer/utility_services.php'),
('10000000-0000-4000-8000-000000000006','City of Manassas Report a Concern','City of Manassas, Virginia','https://user.govoutreach.com/manassascityva/support.php?cmd=shell'),
('10000000-0000-4000-8000-000000000007','City of Manassas Stormwater','City of Manassas, Virginia','https://www.manassasva.gov/engineering/all_about_stormwater/report_a_problem.php')
ON CONFLICT (id) DO NOTHING;

INSERT INTO services (id,provider_id,name,description,category,official_url,source_url,instructions)
SELECT id::uuid,provider_id::uuid,name,description,category,official_url,source_url,instructions
FROM (VALUES
('20000000-0000-4000-8000-000000000010','10000000-0000-4000-8000-000000000003','Report a power outage',
 'Find how to report an electric power outage to the City electric utility.','Electricity',
 'https://www.manassasva.gov/electric_water___sewer/index.php',
 'https://www.manassasva.gov/electric_water___sewer/index.php',
 'The City asks customers to report power outages by calling 703-257-8353, its 24-hour operations center. It also asks customers to keep the phone number on their utility account current so outage calls can be matched to an address; the official page links a form for this. This guide cannot submit requests to the City.'),
('20000000-0000-4000-8000-000000000011','10000000-0000-4000-8000-000000000004','Report a water or sewer problem',
 'Find how to report a water or sewer problem, including after-hours utility emergencies.','Water & sewer',
 'https://www.manassasva.gov/city_departments/water___sewer/sewer/index.php',
 'https://www.manassasva.gov/how_do_i_/view/contacts/index.php',
 'For an after-hours utility emergency, the City lists (703) 257-8353. Non-emergency problems can be reported through the City''s Report a Concern portal, which has Water and Sewer topics. The City maintains the public sewer main; the homeowner is responsible for their private sewer lateral. This guide cannot submit requests to the City.'),
('20000000-0000-4000-8000-000000000012','10000000-0000-4000-8000-000000000005','Pay a utility bill or start utility service',
 'Pay your City electric, water and sewer bill, or find what is needed to start utility service.','Other services',
 'https://ebiz.ci.manassas.va.us/',
 'https://www.manassasva.gov/electric_water___sewer/utility_services.php',
 'Utility bills can be paid online through the City bill pay site. To start service, the City requires a completed application, valid identification and a deposit ($150 for electric service, $100 for water and sewer service). The deposit can be waived with a passing utility credit check or an acceptable letter of credit. See the official page for applications and contact options.'),
('20000000-0000-4000-8000-000000000013','10000000-0000-4000-8000-000000000006','Report a pothole, sidewalk, sign or streetlight problem',
 'Report a street, pothole, sidewalk, curb, street sign or streetlight problem to the City online.','Streets & sidewalks',
 'https://user.govoutreach.com/manassascityva/support.php?cmd=shell',
 'https://user.govoutreach.com/manassascityva/support.php?cmd=shell',
 'Use the City''s Report a Concern portal, linked from the City homepage. Its topics include Street or Pothole Repair, Sidewalk or Curb Repair, Street Signage and Street Lighting. This guide cannot submit requests to the City.'),
('20000000-0000-4000-8000-000000000014','10000000-0000-4000-8000-000000000007','Report a stormwater or drainage problem',
 'Report a non-emergency stormwater or drainage problem to the City.','Water & sewer',
 'https://www.manassasva.gov/engineering/all_about_stormwater/report_a_problem.php',
 'https://www.manassasva.gov/engineering/all_about_stormwater/report_a_problem.php',
 'Use the City''s stormwater problem report for non-emergency issues. Do not use it for emergency flooding: call 9-1-1. For stormwater emergencies, the City lists its 24/7 Operations Center at 703-257-8353. This guide cannot submit requests to the City.')
) AS tasks(id,provider_id,name,description,category,official_url,source_url,instructions)
ON CONFLICT (id) DO NOTHING;
