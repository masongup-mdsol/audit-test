use rusoto_core;
use rusoto_kinesis;
use uuid;
use serde_json;
use chrono;
use rand;

use self::rusoto_kinesis::*;
use self::uuid::Uuid;
use self::rand::prelude::*;
use self::rand::distributions::Alphanumeric;
use std::thread;
use std::path::PathBuf;

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

pub fn create_audits_threaded(threads: i32, audits: i32, verbose: bool, save_path_option: Option<PathBuf>) {
    let start_time = chrono::Local::now();
    println!("Starting create at {}", start_time.to_rfc2822());

    let thread_handles: Vec<thread::JoinHandle<_>> = (0..threads).map(|thread_num| {
        thread::spawn(move || { create_audits_grouped(audits, thread_num, verbose) })
    }).collect();

    let mut all_uuids = vec![];
    for handle in thread_handles {
        let mut thread_uuuids = handle.join().unwrap();
        all_uuids.append(&mut thread_uuuids);
    }

    let end_time = chrono::Local::now();
    println!("Ending create at {}", end_time.to_rfc2822());
    let secs_duration = end_time.signed_duration_since(start_time).num_milliseconds() as f64 / 1000.0;
    let total_audits = threads * audits;
    println!(
        "Created total of {} audits in {:.3} seconds, {:.3} audits/sec, but {} worked",
        total_audits,
        secs_duration,
        total_audits as f64 / secs_duration,
        all_uuids.len()
    );
    if let Some(save_path) = save_path_option {
        let mut file_contents: String = all_uuids.join("\n");
        file_contents.push_str("\n");
        std::fs::write(save_path, file_contents).expect("Unable to write to file");
    }
}

pub fn show_audit_size(show_audit: bool) {
    let audit = create_fake_audit();
    if show_audit {
        let audit_json = serde_json::to_string(&audit).unwrap();
        println!("{}", audit_json);
    }
    else {
        let audit_vec = serde_json::to_vec(&audit).unwrap();
        println!("The audits that we're creating are {} bytes", audit_vec.len());
    }
}

#[allow(dead_code)]
fn create_audits_singly(audits: i32, thread_num: i32) {
    let client = KinesisClient::new(rusoto_core::region::Region::UsEast1);
    let mut write_count = 0;
    for _ in 0..audits {
        let success = client.put_record(PutRecordInput {
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

fn create_audits_grouped(audits: i32, thread_num: i32, verbose: bool) -> Vec<String> {
    let group_of_500_count = audits / 500;
    let remainder = audits % 500;
    let mut uuid_vec = vec![];
    for _ in 0..group_of_500_count {
        uuid_vec.append(&mut create_audit_batch(500, thread_num, verbose));
    }
    if remainder > 0 {
        uuid_vec.append(&mut create_audit_batch(remainder, thread_num, verbose));
    }
    uuid_vec
}

fn create_audit_batch(audits: i32, thread_num: i32, verbose: bool) -> Vec<String> {
    let client = KinesisClient::new(rusoto_core::region::Region::UsEast1);
    let fake_audits: Vec<Audit> = (0..audits).map(|_| create_fake_audit()).collect();
    let audit_uuids: Vec<String> = fake_audits.iter().map(|a| a.audit_uuid.clone()).collect();
    let res = client.put_records(PutRecordsInput {
        stream_name: "audits_persisted_sandbox".to_string(),
        records: fake_audits.iter().map(|a| PutRecordsRequestEntry {
                data: serde_json::to_vec(&a).unwrap(),
                partition_key: a.audit_uuid.clone(),
                ..Default::default()
            }).collect(),
    }).sync().expect(&format!("Kinesis put_records request on thread {}", thread_num));
    if verbose {
        println!("Created {} kinesis records from thread {}, {} failed",
                 res.records.len(), thread_num, res.failed_record_count.unwrap_or(0)
        );
    }
    audit_uuids.into_iter().zip(res.records.into_iter())
        .filter(|(_, kr)| kr.error_code.is_none())
        .map(|(id, _)| id)
        .collect()
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
    fn rand_string(len: usize) -> String {
        let mut rng = thread_rng();
        std::iter::repeat(()).map(|()| rng.sample(Alphanumeric)).take(len).collect()
    }
    (0..count).map(|num| Change {
        field: format!("field_{}", num),
        old: rand_string(100),
        new: rand_string(100),
    }).collect()
}
