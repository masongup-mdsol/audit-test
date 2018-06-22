extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate chrono;

use self::rusoto_dynamodb::*;
use std::collections::{HashSet, HashMap};

pub fn retrieve_audits(what_uri_count: i64, verbose: bool) {
    let client = DynamoDbClient::simple(rusoto_core::region::Region::UsEast1);

    let scan_res = client.scan(&ScanInput {
        table_name: "audits.sandbox".to_string(),
        projection_expression: Some("what_uri".to_string()),
        limit: Some(what_uri_count),
        ..Default::default()
    }).sync().expect("Error in scan");

    let what_uri_set =
        scan_res.items.unwrap().iter().map(|i| i.get("what_uri").unwrap().s.clone().unwrap()).collect();

    audit_query_loop(&client, what_uri_set, verbose);
}

fn audit_query_loop(client: &DynamoDbClient, what_uri_set: HashSet<String>, verbose: bool) {
    let mut total_retrieved = 0;
    let mut total_read_cap_units = 0.0;

    let start_time = chrono::Local::now();
    println!("Starting retrieval at {}", start_time.to_rfc2822());

    'item_loop: for item_uri in what_uri_set {
        let mut items_vec = vec![];
        let mut exclusive_start_key = None;
        loop {
            let query_res = client.query(&build_query_input(item_uri.clone(), exclusive_start_key)).sync();
            match query_res {
                Ok(result) => {
                    if let Some(mut response_vec) = result.items {
                        items_vec.append(&mut response_vec);
                    }
                    total_read_cap_units += result.consumed_capacity.unwrap().capacity_units.unwrap();
                    exclusive_start_key = result.last_evaluated_key;
                    if exclusive_start_key.is_none() {
                        break;
                    }
                }
                Err(error) => {
                    println!("Received error of type {}", error);
                    break 'item_loop;
                }
            }
        }
        total_retrieved += items_vec.len();
        if verbose {
            println!("Got {} items in the query for {}", items_vec.len(), item_uri);
        }
    }

    let end_time = chrono::Local::now();
    println!("Ending create at {}", end_time.to_rfc2822());
    let secs_duration = end_time.signed_duration_since(start_time).num_milliseconds() as f64 / 1000.0;
    println!(
        "Got grand total of {} audits with {} read cap units in {} seconds, {:.3} audits/sec",
        total_retrieved,
        total_read_cap_units,
        secs_duration,
        total_retrieved as f64 / secs_duration
    );
}

fn build_query_input(what_uri: String, exclusive_start_key: Option<HashMap<String, AttributeValue>>)
    -> QueryInput
{
    QueryInput {
        table_name: "audits.sandbox".to_string(),
        index_name: Some("what_uri-when_audited-index".to_string()),
        key_condition_expression: Some("what_uri = :item_uri".to_string()),
        select: Some("ALL_ATTRIBUTES".to_string()),
        exclusive_start_key: exclusive_start_key,
        return_consumed_capacity: Some("INDEXES".to_string()),
        expression_attribute_values: Some([
            (":item_uri".to_string(), AttributeValue { s: Some(what_uri), ..Default::default() })
        ].iter().cloned().collect()),
        ..Default::default()
    }
}
