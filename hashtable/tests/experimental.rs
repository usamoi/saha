use hashtable::experimental::extendible_hashtable::ExtendibleHashtable;
use hashtable::experimental::stack_hashtable::StackHashtable;
use hashtable::hashtable::Hashtable;
use rand::Rng;
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[test]
fn count_distinct_batch() {
    let mut sequence = vec![0u32; 1 << 26];
    sequence.fill_with(|| rand::thread_rng().gen_range(1..1 << 30));
    let mut hashtable = Hashtable::<u32, u32>::new();
    let mut other = Hashtable::<u32, u32>::new();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let start = Instant::now();
            (|| {
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
            })();
            let end = Instant::now();
            println!("time_base = {:?}", end - start);
        });
        scope.spawn(|| {
            let start = Instant::now();
            (|| unsafe {
                const CHUNK: usize = 4096;
                let dels = &[1u32; CHUNK];
                for keys in sequence.chunks(CHUNK) {
                    let dels = &dels[..keys.len()];
                    other.batch_insert::<8, _, _, _>(|d| d, |x, d| x + d, keys, dels);
                }
            })();
            let end = Instant::now();
            println!("time_batch = {:?}", end - start);
        });
    });
    assert_eq!(hashtable.len(), other.len(), "seq = {:?}", sequence);
    let mut repeat = HashSet::new();
    for (key, value) in other.iter() {
        assert!(repeat.insert(key), "seq = {:?}", sequence);
        assert_eq!(hashtable.get(key), Some(value), "seq = {:?}", sequence);
    }
}

#[test]
fn count_distinct_stack() {
    let mut sequence = vec![0u64; 8];
    sequence.fill_with(|| rand::thread_rng().gen_range(0..1 << 30));
    let mut base_faster = 0u32;
    let mut stack_faster = 0u32;
    for _ in 0..10000000 {
        let mut hashtable = Hashtable::<u64, u64>::with_capacity(16);
        let mut other = StackHashtable::<u64, u64, 16>::new();
        let start = Instant::now();
        (|| {
            for &s in sequence.iter() {
                match unsafe { hashtable.insert(s) } {
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
        let time_base = end - start;
        // println!("time_base = {:?}", end - start);
        // println!("cap_base = {:?}", hashtable.capacity());
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
        let time_stack = end - start;
        // println!("time_stack = {:?}", end - start);
        // println!("cap_stack = {:?}", other.capacity());
        assert_eq!(hashtable.len(), other.len(), "seq = {:?}", sequence);
        let mut repeat = HashSet::new();
        for (key, value) in other.iter() {
            assert!(repeat.insert(key), "seq = {:?}", sequence);
            let test = hashtable.get(key).unwrap();
            assert_eq!(test, value, "seq = {:?}", sequence);
        }
        if time_base <= time_stack {
            base_faster += 1;
        } else {
            stack_faster += 1;
        }
    }
    println!("base_faster = {base_faster}");
    println!("stack_faster = {stack_faster}");
}

#[test]
fn count_distinct_extendible() {
    let mut sequence = vec![0u64; 1 << 26];
    sequence.fill_with(|| rand::thread_rng().gen_range(0..1 << 30));
    let mut hashtable = Hashtable::<u64, u64>::new();
    let mut other = ExtendibleHashtable::<u64, u64>::new();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(500));
            let start = Instant::now();
            (|| {
                for &s in sequence.iter() {
                    match unsafe { hashtable.insert(s) } {
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
            println!("time_base = {:?}", end - start);
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
            println!("time_extendible = {:?}", end - start);
            println!("buckets = {:?}", other.buckets());
        });
    });
    assert_eq!(hashtable.len(), other.len(), "seq = {:?}", sequence);
    let mut repeat = HashSet::new();
    for (key, value) in other.iter() {
        assert!(repeat.insert(key), "seq = {:?}", sequence);
        let test = hashtable.get(key).unwrap();
        assert_eq!(test, value, "seq = {:?}", sequence);
    }
}
