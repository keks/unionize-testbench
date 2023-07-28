use unionize_testbench::{
    experiments,
    scenarios::dynamic::{SimObject, Simulator, Trace, TraceEntryRecord},
};

fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("out")?;

    let (tx, rx) = std::sync::mpsc::channel();
    let handles: Vec<_> =
        (0..4)
            .into_iter()
            .map(|i| {
                let tx = tx.clone();
                std::thread::spawn(move || -> std::io::Result<()> {
                    let seed = [0u8; 32];
                    match i {
                        0 => {
                            let trace =
                                experiments::timestamped::timestamped_experiment::<3, 4>(seed);
                            write_trace_to_file::<experiments::timestamped::TimestampSim>(
                                "out/timestamped_3_4.csv",
                                trace,
                            )?;
                        }
                        1 => {
                            let trace =
                                experiments::timestamped::timestamped_experiment_dynamic_split::<4>(
                                    seed,
                                );
                            write_trace_to_file::<experiments::timestamped::TimestampSim>(
                                "out/timestamped_dyn_4.csv",
                                trace,
                            )?;
                        }
                        2 => {
                            let trace = experiments::uniform::uniform_experiment::<3, 4>(seed);
                            write_trace_to_file::<experiments::uniform::UniformSim>(
                                "out/uniform_3_4.csv",
                                trace,
                            )?;
                        }
                        3 => {
                            let trace = experiments::uniform::uniform_experiment::<2, 2>(seed);
                            write_trace_to_file::<experiments::uniform::UniformSim>(
                                "out/uniform_2_2.csv",
                                trace,
                            )?;
                        }
                        _ => unreachable!(),
                    }
                    tx.send(i).unwrap();
                    Ok(())
                })
            })
            .collect();

    let mut running = 4;
    while running > 0 {
        match rx.recv() {
            Ok(0) => println!("timestamped_3_4 done"),
            Ok(1) => println!("timestamped_dyn_4 done"),
            Ok(2) => println!("uniform_3_4 done"),
            Ok(3) => println!("uniform_2_2 done"),
            x => unreachable!("{x:?}"),
        }
        running -= 1;
    }

    for (i, handle) in handles.into_iter().enumerate() {
        if let Err(e) = handle.join() {
            println!("error in thread {i}: {e:?}")
        }
    }

    Ok(())
}

fn write_trace_to_file<S>(path: &str, trace: Trace<S::Item, SimObject>) -> std::io::Result<()>
where
    S: Simulator,
    SimObject: unionize::Object<S::Item>,
{
    let f = std::fs::File::create(path)?;
    let mut wtr = csv::WriterBuilder::new().flexible(true).from_writer(f);

    for (meta, entry) in trace.entries() {
        let rec: TraceEntryRecord<S> = entry.clone().into();
        wtr.serialize((meta, rec)).unwrap();
    }

    Ok(())
}
