use common_hashtable::HashMap;
use hashtable::hashtable::Hashtable;
use rand::Rng;
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[test]
fn count_distinct_normal() {
    let mut sequence = vec![0u64; 1 << 26];
    sequence.fill_with(|| rand::thread_rng().gen_range(0..1 << 30));
    let mut hashtable = HashMap::<u64, u64>::create();
    let mut other = Hashtable::<u64, u64>::new();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(500));
            let start = Instant::now();
            (|| {
                for &s in sequence.iter() {
                    let mut inserted = false;
                    let entry = hashtable.insert_key(&s, &mut inserted);
                    if inserted {
                        entry.set_value(1);
                    } else {
                        *entry.get_mut_value() += 1;
                    }
                }
            })();
            let end = Instant::now();
            println!("time_base = {:?}", end - start);
            println!("cap_base = {:?}", hashtable.capacity());
        });
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(500));
            let start = Instant::now();
            (|| {
                for &s in sequence.iter() {
                    match unsafe { other.insert(s) } {
                        Ok(x) => {
                            x.write(1);
                        }
                        Err(x) => {
                            *x += 1;
                        }
                    }
                }
            })();
            let end = Instant::now();
            println!("time_normal = {:?}", end - start);
            println!("cap_normal = {:?}", other.capacity());
        });
    });
    assert_eq!(hashtable.len(), other.len(), "seq = {:?}", sequence);
    let mut repeat = HashSet::new();
    for (key, value) in other.iter() {
        assert!(repeat.insert(key), "seq = {:?}", sequence);
        let test = hashtable.find_key(key).unwrap().get_value();
        assert_eq!(test, value, "seq = {:?}", sequence);
    }
}
