extern crate rusoto_core;
extern crate rusoto_dynamodb;
extern crate chrono;

use self::rusoto_dynamodb::*;
use std::collections::{HashSet, HashMap};

pub fn retrieve_audits(what_uri_count: i64, verbose: bool) {
    let client = DynamoDbClient::new(rusoto_core::region::Region::UsEast1);

    let scan_res = client.scan(ScanInput {
        table_name: "audits.sandbox".to_string(),
        projection_expression: Some("what_uri_uuid".to_string()),
        limit: Some(what_uri_count),
        ..Default::default()
    }).sync().expect("Error in scan");

    let what_uri_uuid_set =
        scan_res.items.unwrap().iter().map(|i| i.get("what_uri_uuid").unwrap().s.clone().unwrap()).collect();

    audit_query_loop(&client, what_uri_uuid_set, verbose);
}

pub fn retrieve_audit_by_id(audit_id: String) {
    let client = DynamoDbClient::new(rusoto_core::region::Region::UsEast1);

    let item_result = client.get_item(GetItemInput {
        table_name: "audits.sandbox".to_string(),
        key: [("uuid".to_string(), AttributeValue { s: Some(audit_id), ..Default::default() })].iter().cloned().collect(),
        projection_expression: Some("when_audited".to_string()),
        ..Default::default()
    }).sync().expect("Could not get");

    println!("Got results: {}", item_result.item.expect("no item")
             .get("when_audited").unwrap().n.clone().unwrap());
}

pub fn retrieve_by_ids_from_file(file_path: String) {
    let raw_file_data = std::fs::read(&std::path::Path::new(&file_path)).expect("Unable to read file");
    let file_string = String::from_utf8_lossy(&raw_file_data);
    let mut audits_missing = false;
    for file_id_batch in file_string.lines().collect::<Vec<&str>>().chunks(100) {
        if retrieve_many_audits_by_id(file_id_batch) {
            audits_missing = true;
        }
    }
    if audits_missing {
        println!("Missing some audits!");
    }
    else {
        println!("Retrieved all expected audits!");
    }
}

fn retrieve_many_audits_by_id(audit_ids: &[&str]) -> bool {
    let client = DynamoDbClient::new(rusoto_core::region::Region::UsEast1);

    let num_audits = audit_ids.len();
    let table_name = "audits.sandbox";
    let keys = audit_ids.into_iter().map(|id| {
        [(
            "uuid".to_string(),
            AttributeValue { s: Some(id.to_string()), ..Default::default() }
        )].iter().cloned().collect()
    }).collect();

    let mut request_items = HashMap::new();
    request_items.insert(table_name.to_string(), KeysAndAttributes {
        keys,
        ..Default::default()
    });

    let batch_result = client.batch_get_item(BatchGetItemInput {
        request_items: request_items,
        ..Default::default()
    }).sync().unwrap();
    let num_responses = batch_result.responses.unwrap().get(table_name).unwrap().len();
    println!("Got responses: {} to {} audits requested", num_responses, num_audits);
    num_responses != num_audits
}

fn audit_query_loop(client: &DynamoDbClient, what_uri_uuid_set: HashSet<String>, verbose: bool) {
    let mut total_retrieved = 0;
    let mut total_read_cap_units = 0.0;

    let start_time = chrono::Local::now();
    println!("Starting retrieval at {}", start_time.to_rfc2822());

    'item_loop: for item_uuid in what_uri_uuid_set {
        let mut items_vec = vec![];
        let mut exclusive_start_key = None;
        loop {
            let query_res = client.query(build_query_input(item_uuid.clone(), exclusive_start_key)).sync();
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
            println!("Got {} items in the query for {}", items_vec.len(), item_uuid);
        }
    }

    let end_time = chrono::Local::now();
    println!("Ending retrieval at {}", end_time.to_rfc2822());
    let secs_duration = end_time.signed_duration_since(start_time).num_milliseconds() as f64 / 1000.0;
    println!(
        "Got grand total of {} audits with {} read cap units in {} seconds, {:.3} audits/sec",
        total_retrieved,
        total_read_cap_units,
        secs_duration,
        total_retrieved as f64 / secs_duration
    );
}

fn build_query_input(what_uri_uuid: String, exclusive_start_key: Option<HashMap<String, AttributeValue>>)
    -> QueryInput
{
    QueryInput {
        table_name: "audits.sandbox".to_string(),
        index_name: Some("what_uri_uuid-when_audited-index".to_string()),
        key_condition_expression: Some("what_uri_uuid = :item_uuid".to_string()),
        select: Some("ALL_ATTRIBUTES".to_string()),
        exclusive_start_key: exclusive_start_key,
        return_consumed_capacity: Some("INDEXES".to_string()),
        expression_attribute_values: Some([
            (":item_uuid".to_string(), AttributeValue { s: Some(what_uri_uuid), ..Default::default() })
        ].iter().cloned().collect()),
        ..Default::default()
    }
}
