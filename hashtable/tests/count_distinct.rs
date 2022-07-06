use hashtable::hashtable::Hashtable;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[test]
fn count_distinct() {
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
    println!("count_distinct_hashbrown = {:?}", end - start);
    let start = Instant::now();
    let mut saha = Hashtable::new();
    for s in sequence.iter() {
        match saha.insert(&s) {
            Ok(e) => *e = 1,
            Err(e) => *e += 1,
        }
    }
    let end = Instant::now();
    println!("count_distinct_saha = {:?}", end - start);
    assert_eq!(hashbrown.len(), saha.len());
    let mut repeat = HashSet::new();
    for (key, value) in saha.iter() {
        assert!(repeat.insert(key));
        assert_eq!(hashbrown.get(key.as_ref()).copied(), Some(value));
    }
}
