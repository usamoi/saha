use hashtable::batch_hashtable::BatchHashtable;
use hashtable::hashtable::Hashtable;
use rand::Rng;
use std::collections::HashSet;
use std::time::Instant;

#[test]
fn count_distinct_batch() {
    let mut sequence = vec![0; 1 << 24];
    sequence.fill_with(|| rand::thread_rng().gen_range(0..1 << 12));
    let start = Instant::now();
    let mut hashtable = Hashtable::<u64, u64>::new();
    for &s in sequence.iter() {
        match unsafe { hashtable.insert(s) } {
            Ok(ok) => {
                ok.write(1);
            }
            Err(err) => {
                *err += 1;
            }
        }
    }
    let end = Instant::now();
    println!("count_distinct_normal = {:?}", end - start);
    let start = Instant::now();
    let mut batch_hashtable = BatchHashtable::<u64, u64>::new();
    let mut batch = unsafe { batch_hashtable.batch_update::<4, u64, _, _>(|d| d, |x, d| x + d) };
    for &s in sequence.iter() {
        batch.push(s, 1);
    }
    batch.flush();
    let end = Instant::now();
    println!("count_distinct_batch = {:?}", end - start);
    assert_eq!(
        hashtable.len(),
        batch_hashtable.len(),
        "seq = {:?}",
        sequence
    );
    let mut repeat = HashSet::new();
    for (key, value) in batch_hashtable.iter() {
        assert!(repeat.insert(key), "seq = {:?}", sequence);
        assert_eq!(hashtable.get(key), Some(value), "seq = {:?}", sequence);
    }
}
