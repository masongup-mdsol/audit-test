extern crate rusoto_core;
extern crate rusoto_dynamodb;

use self::rusoto_dynamodb::*;
use std::collections;

pub fn retrieve_audits(what_uri_count: i64) {
    let client = DynamoDbClient::simple(rusoto_core::region::Region::UsEast1);

    let scan_res = client.scan(&ScanInput {
        table_name: "audits.sandbox".to_string(),
        projection_expression: Some("what_uri".to_string()),
        limit: Some(what_uri_count),
        ..Default::default()
    }).sync().expect("Error in scan");

    let what_uri_set: collections::HashSet<String> =
        scan_res.items.unwrap().iter().map(|i| i.get("what_uri").unwrap().s.clone().unwrap()).collect();

    let mut total_retrieved = 0;
    let mut total_read_cap_units = 0.0;

    for item_uri in what_uri_set {
        let mut items_vec = vec![];
        let mut exclusive_start_key = None;
        loop {
            let query_res = client.query(&QueryInput {
                table_name: "audits.sandbox".to_string(),
                index_name: Some("what_uri-when_audited-index".to_string()),
                key_condition_expression: Some("what_uri = :item_uri".to_string()),
                select: Some("ALL_ATTRIBUTES".to_string()),
                exclusive_start_key: exclusive_start_key,
                return_consumed_capacity: Some("INDEXES".to_string()),
                expression_attribute_values: Some([
                    (":item_uri".to_string(), AttributeValue { s: Some(item_uri.clone()), ..Default::default() })
                ].iter().cloned().collect()),
                ..Default::default()
            }).sync().expect("Error in query");
            if let Some(mut response_vec) = query_res.items {
                items_vec.append(&mut response_vec);
            }
            total_read_cap_units += query_res.consumed_capacity.unwrap().capacity_units.unwrap();
            exclusive_start_key = query_res.last_evaluated_key;
            if exclusive_start_key.is_none() {
                break;
            }
        }
        total_retrieved += items_vec.len();
        println!("Got {} items in the query for {}", items_vec.len(), item_uri);
    }

    println!("Got grand total of {} audits with {} read cap units", total_retrieved, total_read_cap_units);
}
