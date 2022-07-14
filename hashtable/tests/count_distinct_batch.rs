use hashtable::batch_hashtable::BatchHashtable;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[test]
fn count_distinct_demo() {
    let mut sequence = vec![0; 1 << 20];
    sequence.fill_with(|| rand::thread_rng().gen_range(0..1 << 12));
    let start = Instant::now();
    let mut hashbrown = HashMap::<u64, u64>::new();
    for &s in sequence.iter() {
        if let Some(x) = hashbrown.get_mut(&s) {
            *x += 1;
        } else {
            hashbrown.insert(s, 1);
        }
    }
    let end = Instant::now();
    println!("count_distinct_hashbrown = {:?}", end - start);
    let start = Instant::now();
    let mut saha = BatchHashtable::<u64, u64>::new();
    let mut batch = unsafe { saha.batch_update(|d| d, |x, d| x + d) };
    for &s in sequence.iter() {
        batch.push(s, 1);
    }
    batch.flush();
    let end = Instant::now();
    println!("count_distinct_saha = {:?}", end - start);
    assert_eq!(hashbrown.len(), saha.len(), "seq = {:?}", sequence);
    let mut repeat = HashSet::new();
    for (key, value) in saha.iter() {
        assert!(repeat.insert(key), "seq = {:?}", sequence);
        assert_eq!(hashbrown.get(key), Some(value), "seq = {:?}", sequence);
    }
}
