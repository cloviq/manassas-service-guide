// All descriptive fixture values are fictional; source keys/URLs test adapter schemas.
use serde_json::json;
use service_guide::{
    api::Service,
    projection::{page, project},
};
fn row(id: u128, name: &str, source: &str, kind: &str) -> Service {
    serde_json::from_value(json!({"id":uuid::Uuid::from_u128(id),"name":name,"description":"Street","category":"Streets & sidewalks","provider_name":"Publisher","jurisdiction":"City","official_url":"https://example.org","source_url":format!("https://example.org/{id}"),"instructions":"Verify source","record_type":kind,"source_key":source,"source_record_id":id.to_string(),"attributes":{"private":"never expose"},"geometry":{"paths":[[[id,0],[id,1]]]}})).unwrap()
}
#[test]
fn expansion_details_are_safe_and_leaf_year_is_explicitly_unknown() {
    let mut leaf = row(1, "Section 1", "Leaf_Collection_Sections/0", "place");
    leaf.attributes = json!({"NAME":"Section 1","DESCRIPT":"October 5–9","SCHEDULE":"Seasonal","LASTUPDATE":1600000000000_i64,"Contact":"private","Notes":"private"});
    let d = service_guide::projection::details(&leaf);
    assert_eq!(d["DESCRIPT"], "October 5–9");
    assert!(
        d["schedule_caveat"]
            .as_str()
            .unwrap()
            .contains("year not stated")
    );
    assert!(!d.contains_key("Contact"));
    let mut bulk = row(2, "Bulk", "Manassas_Bulk_Waste_Collection/0", "place");
    bulk.attributes = json!({"Agency":"Example Waste Operator (fictional)","Phone":"+1-202-555-0100","Website2":"http://unsafe.example","Contact":"private"});
    let d = service_guide::projection::details(&bulk);
    assert_eq!(d["Agency"], "Example Waste Operator (fictional)");
    assert!(!d.contains_key("Website2"));
    assert!(!d.contains_key("Contact"));
    assert!(
        service_guide::projection::links(&bulk)
            .source_record
            .contains("outFields=OBJECTID%2CDistrictName2%2CAgency%2CPhone%2CWebsite2")
    );
    let mut bike = row(3, "Bike", "Bike_Facilities_and_Racks/1", "route");
    bike.attributes = json!({"Miles":0.25,"Status":"Proposed","NOTES":"private"});
    let d = service_guide::projection::details(&bike);
    assert_eq!(d["Miles"], 0.25);
    assert_eq!(d["Status"], "Proposed");
    assert!(!d.contains_key("NOTES"));
}
#[test]
fn published_street_details_are_typed_allowlisted_and_record_links_are_specific() {
    let mut record = row(1, "EXAMPLE", "Centerline_COM/0", "route");
    record.official_url = "https://gis-cityofmanassas.hub.arcgis.com".into();
    record.source_url = "https://services1.arcgis.com/3wpOgOChiWXPeFWB/arcgis/rest/services/Centerline_COM/FeatureServer/0/query?f=json&objectIds=1&outFields=%2A&returnGeometry=true&outSR=4326".into();
    record.attributes = json!({"FULLSTREET":"EXAMPLE DR","TYPE":"DR","SPEEDLIMIT":25,"L_F_ADD":10315,"L_T_ADD":10315,"R_F_ADD":10312,"R_T_ADD":10314,"ZIPL":"00000","LastEditor":"staff","Creator":"staff","NOTES":"private","Phone":"+1-202-555-0100"});
    let value = serde_json::to_value(project(vec![record])).unwrap();
    let r = &value[0];
    assert_eq!(r["details"]["FULLSTREET"], "EXAMPLE DR");
    assert_eq!(r["details"]["SPEEDLIMIT"], 25);
    assert!(r["details"].get("LastEditor").is_none());
    assert!(r["details"].get("Phone").is_none());
    assert_eq!(r["official_url"], r["source_url"]);
    assert_eq!(r["links"]["source_record"], r["source_url"]);
    assert!(r["links"]["service_action"].is_null());
    assert!(r["description"].as_str().unwrap().contains("EXAMPLE DR"));
    assert_eq!(r["segments"][0]["details"]["FULLSTREET"], "EXAMPLE DR");
}
#[test]
fn other_source_details_use_real_fields_without_staff_metadata() {
    for (source, attrs, key, expected) in [
        (
            "Snow_Routes/0",
            json!({"NAME":"Example One","TYPE":"Ct"}),
            "TYPE",
            json!("Ct"),
        ),
        (
            "Snow_Routes/3",
            json!({"NAME":"Example Two","TYPE":"St","L_F_ADD":101}),
            "L_F_ADD",
            json!(101),
        ),
        (
            "Manassas_Government_Services_WFL1/0",
            json!({"Address2":"123 Example Avenue (fictional)","Phone":"+1-202-555-0100"}),
            "Address2",
            json!("123 Example Avenue (fictional)"),
        ),
        (
            "watermain_breaks_view/0",
            json!({"LOCDESC":"123 Example Avenue (fictional)","INCIDENTDATE":946684800000_i64}),
            "INCIDENTDATE",
            json!(946684800000_i64),
        ),
        (
            "COM_StormData_Public_View/3",
            json!({"Location":"Example Road","DischType":"OUTFALL","LastEditor":"staff","Creator":"staff","GPSWho":"staff","Comments":"private"}),
            "Location",
            json!("Example Road"),
        ),
        (
            "COM_StormData_Public_View/8",
            json!({"FacName":"Example Pond (fictional)","BMP_Type":"Pond"}),
            "FacName",
            json!("Example Pond (fictional)"),
        ),
        (
            "Manassas_Bulk_Waste_Collection/0",
            json!({"DistrictName2":"Bulk Waste","Contact":"staff","Agency":"not verified"}),
            "DistrictName2",
            json!("Bulk Waste"),
        ),
    ] {
        let mut record = row(1, "Source name", source, "route");
        record.attributes = attrs;
        let value = serde_json::to_value(project(vec![record])).unwrap();
        assert_eq!(value[0]["details"][key], expected, "{source}");
        for forbidden in ["LastEditor", "Creator", "GPSWho", "Comments", "Contact"] {
            assert!(value[0]["details"].get(forbidden).is_none());
        }
        if source.starts_with("Snow_Routes/") {
            assert!(
                !value[0]["description"]
                    .as_str()
                    .unwrap()
                    .contains("Source name")
            );
            assert!(
                value[0]["description"]
                    .as_str()
                    .unwrap()
                    .contains("Example One Ct")
                    || value[0]["description"]
                        .as_str()
                        .unwrap()
                        .contains("Example Two St")
            );
        }
    }
}
#[test]
fn grouped_details_never_claim_one_segments_range_for_the_whole_street() {
    let mut a = row(1, "EXAMPLE", "Centerline_COM/0", "route");
    let mut b = row(2, "EXAMPLE", "Centerline_COM/0", "route");
    a.attributes = json!({"FULLSTREET":"EXAMPLE DR","SPEEDLIMIT":25,"L_F_ADD":10315});
    b.attributes = json!({"FULLSTREET":"EXAMPLE DR","SPEEDLIMIT":35,"L_F_ADD":10401});
    let result = project(vec![a, b]);
    assert_eq!(result[0].details["FULLSTREET"], "EXAMPLE DR");
    assert!(!result[0].details.contains_key("SPEEDLIMIT"));
    assert!(!result[0].details.contains_key("L_F_ADD"));
    assert_eq!(result[0].segments[1].details["SPEEDLIMIT"], 35);
}
#[test]
fn malformed_or_unknown_details_fail_closed() {
    let mut r = row(1, "Street", "Centerline_COM/0", "route");
    r.attributes = json!({"FULLSTREET":false,"TYPE":{},"SPEEDLIMIT":"25","L_F_ADD":0,"L_T_ADD":-1,"R_F_ADD":1.5,"R_T_ADD":null,"ZIPL":"  ","ZIPR":"bad\nvalue"});
    assert!(service_guide::projection::details(&r).is_empty());
    r.source_key = "Other/0".into();
    r.attributes = json!({"FULLSTREET":"DO NOT EXPOSE","Phone":"+1-202-555-0100"});
    assert!(service_guide::projection::details(&r).is_empty());
}
#[test]
fn source_aware_nonblank_normalization_never_collapses_other_resources() {
    let mut rows = vec![
        row(1, " Example   Road ", "Centerline_COM/0", "route"),
        row(2, "EXAMPLE ROAD", "Centerline_COM/0", "route"),
    ];
    for (id, source, kind, name) in [
        (3, "Other/0", "route", "EXAMPLE ROAD"),
        (4, "Centerline_COM/1", "route", "EXAMPLE ROAD"),
        (5, "Centerline_COM/0", "place", "EXAMPLE ROAD"),
        (6, "Centerline_COM/0", "outage", "EXAMPLE ROAD"),
        (7, "Centerline_COM/0", "route", " "),
        (8, "Centerline_COM/0", "route", " "),
        (9, "Other/0", "route", "EXAMPLE ROAD"),
    ] {
        rows.push(row(id, name, source, kind));
    }
    let result = project(rows);
    assert_eq!(result.len(), 8);
    assert_eq!(result[0].segment_count, Some(2));
    assert!(result.iter().skip(1).all(|r| r.segment_count.is_none()));
    let encoded = serde_json::to_value(&result[0]).unwrap();
    assert!(encoded.get("attributes").is_none());
    assert_eq!(encoded["geometry"], serde_json::Value::Null);
    assert_eq!(encoded["segments"].as_array().unwrap().len(), 2);
    assert!(encoded["segments"][0].get("attributes").is_none());
}
#[test]
fn fused_candidates_group_before_pagination_with_stable_ids_and_no_double_count() {
    // Fused candidate order need not be contiguous by street; duplicate UUIDs
    // arriving from retrieval legs never count as an extra physical segment.
    let result = project(vec![
        row(14, "EXAMPLE ROAD", "Centerline_COM/0", "route"),
        row(99, "Other street", "Centerline_COM/0", "route"),
        row(1, "example road", "Centerline_COM/0", "route"),
        row(14, "EXAMPLE ROAD", "Centerline_COM/0", "route"),
    ]);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].record.id, uuid::Uuid::from_u128(1));
    assert_eq!(result[0].segment_count, Some(2));
    let next = page(result, 1, 1);
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].record.name, "Other street");
    assert!(next[0].segments.is_empty());
}
#[test]
fn every_segment_geometry_and_provenance_survives_projection() {
    let result = project(
        (1..=14)
            .map(|id| row(id, "EXAMPLE ROAD", "Centerline_COM/0", "route"))
            .collect(),
    );
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].segment_count, Some(14));
    for (i, s) in result[0].segments.iter().enumerate() {
        assert_eq!(s.source_record_id, (i + 1).to_string());
        assert_eq!(s.source_url, format!("https://example.org/{}", i + 1));
        assert_eq!(s.geometry.as_ref().unwrap()["paths"][0][0][0], i + 1);
    }
}
