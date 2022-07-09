use hashtable::adaptive_hashtable::AdaptiveHashtable;
use rand::Rng;
use std::time::Instant;

fn main() {
    let mut sequence = Vec::new();
    for _ in 0..10000000 {
        let length = rand::thread_rng().gen_range(0..64);
        let mut array = vec![0u8; length];
        rand::thread_rng().fill(&mut array[..]);
        sequence.push(array);
    }
    let start = Instant::now();
    let mut saha = AdaptiveHashtable::new();
    for s in sequence.iter() {
        match unsafe { saha.insert(&s) } {
            Ok(e) => {
                e.write(1);
            }
            Err(e) => {
                *e += 1;
            }
        }
    }
    let end = Instant::now();
    println!("count_distinct_saha = {:?}", end - start);
}
