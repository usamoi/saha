use benchmarker::measure_memory::MeasureMemory;
use benchmarker::measure_time::measure_time;
use benchmarker::subject::Subject;
use common_hashtable::UnsafeBytesRef;
use std::error::Error;
use std::io::Read;

#[global_allocator]
static MEASURE_MEMORY: MeasureMemory = MeasureMemory::new();

fn read<F: FnMut(Vec<Box<[u8]>>)>(
    dataset_files: &[String],
    mut f: F,
) -> Result<(), Box<dyn Error>> {
    let mut strings = Vec::<Box<[u8]>>::new();
    let mut buffer = vec![0u8; 262144].into_boxed_slice();
    let mut cached = Vec::<u8>::new();
    for dataset_file in dataset_files {
        let mut file = std::fs::OpenOptions::new().read(true).open(dataset_file)?;
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    for c in buffer[..n].iter().copied() {
                        match c {
                            b' ' | b',' | b'\n' | b'\r' | b'"' => {
                                if cached.len() != 0 {
                                    strings.push(cached.into_boxed_slice());
                                    cached = Vec::new()
                                }
                                if strings.len() >= 262144 {
                                    f(strings);
                                    strings = Vec::new();
                                }
                            }
                            c => cached.push(c),
                        }
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::Interrupted => continue,
                    _ => panic!(),
                },
            }
        }
        if cached.len() != 0 {
            strings.push(cached.into_boxed_slice());
            cached = Vec::new();
        }
        f(strings);
        strings = Vec::new();
    }
    Ok(())
}

fn solver<S: Subject>(manifest: &Manifest) -> Result<(), Box<dyn Error>> {
    for ManifestItem { name, files } in manifest.iter() {
        let mut time_build = 0usize;
        let mut time_probe = 0usize;
        let mut time_foreach = 0usize;
        let mut subject = S::new();
        read(files, |strings| {
            time_build += measure_time(|| {
                for string in strings {
                    subject.build(string, || 1, |x| *x = *x + 1);
                }
            });
        })?;
        read(files, |strings| {
            time_probe += measure_time(|| {
                for string in strings.iter() {
                    subject.probe(string).expect("incorrect implement");
                }
            });
        })?;
        let mut count = 0u64;
        let mut count_distinct = 0u64;
        time_foreach += measure_time(|| {
            subject.foreach(|(_, v)| {
                count += 1;
                count_distinct += v;
            })
        });
        let before_dropping = MEASURE_MEMORY.measure();
        drop(subject);
        let memory = before_dropping - MEASURE_MEMORY.measure();
        println!(
            "{},{name},{time_build},{time_probe},{time_foreach},{memory},{count},{count_distinct}",
            S::NAME
        );
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct ManifestItem {
    name: String,
    files: Vec<String>,
}

type Manifest = Vec<ManifestItem>;

#[derive(clap::Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(short, long, value_parser)]
    path: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = <Args as clap::Parser>::parse();
    if let Some(path) = args.path {
        std::env::set_current_dir(path)?;
    }
    let manifest = serde_json::from_str::<Manifest>(&std::fs::read_to_string("manifest.json")?)?;
    println!("subject,dataset,time_build,time_probe,time_foreach,memory,count,count_distinct");
    solver::<hashtable::adaptive_hashtable::AdaptiveHashtable<u64>>(&manifest)?;
    solver::<(common_hashtable::HashMap<UnsafeBytesRef, u64>, Vec<Box<[u8]>>)>(&manifest)?;
    solver::<hashbrown::HashMap<Box<[u8]>, u64>>(&manifest)?;
    Ok(())
}
