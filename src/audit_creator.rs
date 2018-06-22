extern crate rusoto_core;
extern crate rusoto_kinesis;
extern crate uuid;
extern crate serde_json;
extern crate chrono;
extern crate rand;

use self::rusoto_kinesis::*;
use self::uuid::Uuid;
use self::rand::Rng;
use std::thread;

#[derive(Debug, Serialize)]
struct Audit {
    audit_uuid: String,
    what_uri: String,
    where_uri: String,
    who_uri: String,
    when_audited: String,
    tags: Vec<String>,
    which_changed: WhichChanged,
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Serialize)]
enum ChangeType {
    create,
    update,
    destroy,
}

#[derive(Debug, Serialize)]
struct WhichChanged {
    #[serde(rename = "type")]
    audit_type: ChangeType,
    changes: Vec<Change>
}

#[derive(Debug, Serialize)]
struct Change {
    field: String,
    old: String,
    new: String,
}

pub fn create_audits_threaded(threads: i32, audits: i32, verbose: bool) {
    let start_time = chrono::Local::now();
    println!("Starting create at {}", start_time.to_rfc2822());

    let thread_handles: Vec<thread::JoinHandle<_>> = (0..threads).map(|thread_num| {
        thread::spawn(move || { create_audits_grouped(audits, thread_num, verbose) })
    }).collect();

    for handle in thread_handles {
        handle.join().unwrap();
    }

    let end_time = chrono::Local::now();
    println!("Ending create at {}", end_time.to_rfc2822());
    let secs_duration = end_time.signed_duration_since(start_time).num_milliseconds() as f64 / 1000.0;
    let total_audits = threads * audits;
    println!(
        "Created total of {} audits in {:.3} seconds, {:.3} audits/sec",
        total_audits,
        secs_duration,
        total_audits as f64 / secs_duration
    );
}

pub fn show_audit_size(show_audit: bool) {
    let audit = create_fake_audit();
    let audit_vec = serde_json::to_vec(&audit).unwrap();
    let audit_json = serde_json::to_string(&audit).unwrap();
    println!("The audits that we're creating are {} bytes", audit_vec.len());
    if show_audit {
        println!("{}", audit_json);
    }
}

#[allow(dead_code)]
fn create_audits_singly(audits: i32, thread_num: i32) {
    let client = KinesisClient::simple(rusoto_core::region::Region::UsEast1);
    let mut write_count = 0;
    for _ in 0..audits {
        let success = client.put_record(&PutRecordInput {
            stream_name: "audits_persisted_sandbox".to_string(),
            partition_key: Uuid::new_v4().to_string(),
            data: serde_json::to_vec(&create_fake_audit()).unwrap(),
            ..Default::default()
        }).sync().is_ok();
        if success {
            write_count += 1;
            if write_count % 100 == 0 {
                println!("Wrote {} audits from thread {}", write_count, thread_num);
            }
        }
        else {
            println!("A request failed");
        }
    }
}

fn create_audits_grouped(audits: i32, thread_num: i32, verbose: bool) {
    let group_of_500_count = audits / 500;
    let remainder = audits % 500;
    for _ in 0..group_of_500_count {
        create_audit_batch(500, thread_num, verbose);
    }
    if remainder > 0 {
        create_audit_batch(remainder, thread_num, verbose);
    }
}

fn create_audit_batch(audits: i32, thread_num: i32, verbose: bool) {
    let client = KinesisClient::simple(rusoto_core::region::Region::UsEast1);
    let res = client.put_records(&PutRecordsInput {
        stream_name: "audits_persisted_sandbox".to_string(),
        records: (0..audits).map(|_| PutRecordsRequestEntry {
                data: serde_json::to_vec(&create_fake_audit()).unwrap(),
                partition_key: Uuid::new_v4().to_string(),
                ..Default::default()
            }).collect(),
    }).sync().expect(&format!("Kinesis put_records request on thread {}", thread_num));
    if verbose {
        println!("Created {} kinesis records from thread {}", res.records.len(), thread_num);
    }
}


fn create_fake_audit() -> Audit {
    Audit {
      audit_uuid: Uuid::new_v4().to_string(),
      what_uri: format!("com:mdsol:test_items:{}", Uuid::new_v4().to_string()),
      where_uri: format!("com:mdsol:test_items:{}", Uuid::new_v4().to_string()),
      who_uri: "com:mdsol:apps:c775584c-7438-11e8-b836-c3b1435e3798".to_string(),
      when_audited: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
      tags: vec![],
      which_changed: WhichChanged {
        audit_type: ChangeType::create,
        changes: create_fake_changes(40),
      }
    }
}

fn create_fake_changes(count: i32) -> Vec<Change> {
    (0..count).map(|num| Change {
        field: format!("field_{}", num),
        old: rand::thread_rng().gen_ascii_chars().take(80).collect(),
        new: rand::thread_rng().gen_ascii_chars().take(80).collect(),
    }).collect()
}
