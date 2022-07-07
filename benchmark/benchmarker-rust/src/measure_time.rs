use std::time::Instant;

pub fn measure_time<F: FnOnce()>(f: F) -> usize {
    let t = Instant::now();
    f();
    t.elapsed().as_millis().try_into().unwrap()
}
