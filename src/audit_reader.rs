extern crate rusoto_core;
extern crate rusoto_dynamodb;

use self::rusoto_dynamodb::*;
use std::collections;

pub fn retrieve_audits() {
    let client = DynamoDbClient::simple(rusoto_core::region::Region::UsEast1);
    //scan to get a bunch of what_uris
    let scan_res = client.scan(&ScanInput {
        table_name: "audits.sandbox".to_string(),
        projection_expression: Some("what_uri".to_string()),
        limit: Some(10),
        ..Default::default()
    }).sync().unwrap();
    let what_uri_set: collections::HashSet<String> =
        scan_res.items.unwrap().iter().map(|i| i.get("what_uri").unwrap().s.clone().unwrap()).collect(); //{
        //let item: &collections::HashMap<String, AttributeValue> = i;
        //let field_option: Option<&AttributeValue> = item.get("what_uri"); //hash get returns an owned option of a ref of the value
        //let field: &AttributeValue = field_option.unwrap(); //we own it and so can unwrap it
        //let value_option: Option<String> = field.s.clone(); //we can access this item as owned only if we clone it
        //value_option.unwrap() //we can only unwrap it if we own it
    //}).collect();

    for item_uri in what_uri_set {
        println!("{}", item_uri);
        let query_res = client.query(&QueryInput {
            table_name: "audits.sandbox".to_string(),
            index_name: Some("what_uri-when_audited-index".to_string()),
            key_condition_expression: Some("what_uri = :item_uri".to_string()),
            select: Some("ALL_ATTRIBUTES".to_string()),
            expression_attribute_values: Some([
                (":item_uri".to_string(), AttributeValue { s: item_uri, ..Default::default() })
            ].iter().cloned().collect()),
            ..Default::default()
        }).sync().unwrap();
    }

    //println!("Calling retrieve audits, got {} items", item_vec.len());
    //let first_item_field = item_vec.get(0).unwrap().get("what_uri").unwrap();
    //let first_item_str = &first_item_field.s;
    //println!("first item is {:?}", first_item_field);
    //if let Some(item_str) = first_item_str {
        //println!("what_uri of first item is {}", item_str);
    //}
}
