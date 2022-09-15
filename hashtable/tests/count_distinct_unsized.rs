use hashtable::unsized_hashtable::UnsizedHashtable;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[test]
fn count_distinct_unsized() {
    let mut sequence = Vec::new();
    for _ in 0..10000000 {
        let length = rand::thread_rng().gen_range(0..64);
        let mut array = vec![0u8; length];
        rand::thread_rng().fill(&mut array[..]);
        sequence.push(array);
    }
    let start = Instant::now();
    let mut hashbrown = HashMap::<&[u8], u64>::new();
    for s in sequence.iter() {
        if let Some(x) = hashbrown.get_mut(&s[..]) {
            *x += 1;
        } else {
            hashbrown.insert(&s, 1);
        }
    }
    let end = Instant::now();
    println!("time_hashbrown = {:?}", end - start);
    let start = Instant::now();
    let mut saha = UnsizedHashtable::<[u8], u64>::new();
    for s in sequence.iter() {
        match unsafe { saha.insert(&s) } {
            Ok(e) => {
                e.write(1u64);
            }
            Err(e) => {
                *e += 1;
            }
        }
    }
    let end = Instant::now();
    println!("time_saha = {:?}", end - start);
    assert_eq!(hashbrown.len(), saha.len());
    let mut repeat = HashSet::new();
    for (key, value) in saha.iter() {
        assert!(repeat.insert(key));
        assert_eq!(hashbrown.get(key.as_ref()), Some(value));
    }
}
