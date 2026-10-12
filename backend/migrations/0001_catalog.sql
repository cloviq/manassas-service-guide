CREATE TABLE providers (
    id uuid PRIMARY KEY,
    name text NOT NULL,
    jurisdiction text NOT NULL,
    official_url text NOT NULL
);
CREATE TABLE services (
    id uuid PRIMARY KEY,
    provider_id uuid NOT NULL REFERENCES providers(id),
    name text NOT NULL,
    description text NOT NULL,
    category text NOT NULL CHECK (category IN ('Trash & recycling','Water & sewer','Electricity','Streets & sidewalks','Other services')),
    official_url text NOT NULL,
    source_url text NOT NULL,
    instructions text NOT NULL
);
CREATE TABLE tickets (
    id uuid PRIMARY KEY,
    service_id uuid REFERENCES services(id),
    description text NOT NULL CHECK (char_length(description) BETWEEN 10 AND 4000),
    contact_email text CHECK (char_length(contact_email) <= 254),
    status text NOT NULL DEFAULT 'new' CHECK (status = 'new'),
    demo boolean NOT NULL DEFAULT true CHECK (demo),
    created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO providers (id,name,jurisdiction,official_url) VALUES
('10000000-0000-4000-8000-000000000001','City of Manassas Refuse & Recycling','City of Manassas, Virginia','https://www.manassasva.gov/public_works/trash___recycling/index.php')
ON CONFLICT (id) DO NOTHING;

-- Small, deliberately bounded catalog summarizing the cited official City page.
-- Do not infer contacts, eligibility, or response SLAs from these descriptions.
INSERT INTO services (id,provider_id,name,description,category,official_url,source_url,instructions)
SELECT id::uuid,'10000000-0000-4000-8000-000000000001'::uuid,name,description,'Trash & recycling',
'https://www.manassasva.gov/public_works/trash___recycling/index.php',
'https://www.manassasva.gov/public_works/trash___recycling/index.php',
'Follow the official City Refuse & Recycling page for current instructions and contact options. This guide cannot submit requests to the City.'
FROM (VALUES
('20000000-0000-4000-8000-000000000001','Report missed trash collection','Find official instructions for reporting a missed trash pickup.'),
('20000000-0000-4000-8000-000000000002','Report missed recycling collection','Find official instructions for reporting a missed recycling pickup.'),
('20000000-0000-4000-8000-000000000003','Report missed yard waste collection','Find official instructions for reporting a missed yard waste pickup.'),
('20000000-0000-4000-8000-000000000004','Arrange a bulk item pickup','Check the City information about bulk item pickups before arranging collection.'),
('20000000-0000-4000-8000-000000000005','Request a replacement cart','Find the City information about replacement trash and recycling carts.'),
('20000000-0000-4000-8000-000000000006','Check Waste Wise collection schedules','Use the City Waste Wise schedule information to check collection days.')
) AS tasks(id,name,description)
ON CONFLICT (id) DO NOTHING;
