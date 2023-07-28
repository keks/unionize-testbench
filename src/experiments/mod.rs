use std::collections::BTreeMap;

use crate::scenarios::dynamic::{Event, Frequency, Probability, SimDuration, SimInstant, Triggers};

fn sleep_schedule(
    party_id: usize,
    offset: SimDuration,
    time_awake: SimDuration,
    awake_probabilities: Vec<(Probability, Event)>,
) -> Triggers {
    let awake_probabilities: Vec<_> = awake_probabilities
        .into_iter()
        .map(|(p, e)| (party_id, p, e))
        .collect();

    let awake_event = Event::repeat(
        SimDuration::DAY,
        Event::AddProbabilities(awake_probabilities.clone()),
    );

    let sleep_event = Event::repeat(
        SimDuration::DAY,
        Event::drop_probabilities(move |(evt_party_id, p, evt)| {
            let mut found = false;
            for i in awake_probabilities.clone() {
                if (*evt_party_id, p.clone(), evt.clone()) == i {
                    found = true;
                }
            }
            found
        }),
    );

    Triggers::new(
        BTreeMap::from_iter(
            [
                (SimInstant::zero() + offset, vec![(party_id, awake_event)]),
                (
                    SimInstant::zero() + offset + time_awake,
                    vec![(party_id, sleep_event)],
                ),
            ]
            .into_iter(),
        ),
        vec![],
    )
}

fn many_parties<F: Fn(usize) -> Triggers>(party_ids: std::ops::Range<usize>, f: F) -> Triggers {
    let mut triggers = Triggers::default();

    for mut party_triggers in party_ids.map(f) {
        triggers.append(&mut party_triggers)
    }

    triggers
}

fn probabilistic_triggers(probabilistic: Vec<(usize, Probability, Event)>) -> Triggers {
    Triggers::new(Default::default(), probabilistic)
}

fn trigger_conf_10() -> Triggers {
    let prob_hourly = Probability::from_frequency(Frequency::from_period(SimDuration::HOUR));
    let prob_every_three_hours =
        Probability::from_frequency(Frequency::from_period(3 * SimDuration::HOUR));
    let prob_daily = Probability::from_frequency(Frequency::from_period(SimDuration::DAY));

    let mut initial_triggers = Triggers::default();
    initial_triggers.append(&mut many_parties(0..4, |party_id| {
        sleep_schedule(
            party_id,
            SimDuration::zero(),
            10 * SimDuration::HOUR,
            vec![
                (prob_hourly, Event::Post),
                (prob_every_three_hours, Event::Sync(8)),
            ],
        )
    }));

    initial_triggers.append(&mut many_parties(4..8, |party_id| {
        sleep_schedule(
            party_id,
            SimDuration::zero(),
            10 * SimDuration::HOUR,
            vec![
                (prob_hourly, Event::Post),
                (prob_every_three_hours, Event::Sync(9)),
            ],
        )
    }));

    initial_triggers.append(&mut probabilistic_triggers(vec![(
        8,
        prob_daily,
        Event::Sync(9),
    )]));

    initial_triggers.append(&mut probabilistic_triggers(vec![(
        9,
        prob_daily,
        Event::Sync(8),
    )]));

    initial_triggers
}

pub mod timestamped {

    use crate::scenarios::tree::mem_rc;
    use rand::SeedableRng;
    use unionize::item::timestamped::TimestampItem;
    use unionize::object::timestamped::TimestampedObject;
    use unionize::protocol::Encodable;
    use unionize::Object;

    use crate::scenarios::dynamic::{self, SimDuration, SimInstant, SimObject, Simulator, Trace};
    use crate::suites::{timestamped, uniform};

    impl unionize::Item for SimInstant {
        fn zero() -> Self {
            SimInstant(0)
        }

        fn next(&self) -> Self {
            SimInstant(self.0 + 1)
        }
    }

    impl TimestampItem for SimInstant {}

    impl TimestampedObject for dynamic::SimObject {
        type Timestamp = SimInstant;
        type Unique = uniform::Item;

        fn to_timestamp(&self) -> Self::Timestamp {
            self.timestamp
        }

        fn to_unique(&self) -> Self::Unique {
            <SimObject as Object<uniform::Item>>::to_item(&self)
        }

        fn validate_self_consistency(&self) -> bool {
            <SimObject as Object<uniform::Item>>::validate_self_consistency(&self)
        }
    }

    #[derive(Clone)]
    pub struct TimestampSim;
    impl Simulator for TimestampSim {
        type Item = timestamped::Item;
        type Monoid = timestamped::Monoid;
        type Node = timestamped::Node;
        type Tree = mem_rc::Tree<Self::Monoid>;
        type EncodedMonoid = <uniform::Monoid as Encodable>::Encoded;
    }

    pub fn timestamped_experiment<const SPLITS: usize, const THRESH: usize>(
        seed: [u8; 32],
    ) -> Trace<timestamped::Item, SimObject> {
        let mut rng = rand_chacha::ChaCha8Rng::from_seed(seed);
        let n_parties = 10;
        let initial_triggers = super::trigger_conf_10();
        let length = 18 * SimDuration::MONTH;

        TimestampSim::sim(
            &mut rng,
            n_parties,
            initial_triggers,
            length,
            timestamped::run_protocol::<_, _, _, SPLITS, THRESH>,
        )
    }

    pub fn timestamped_experiment_dynamic_split<const THRESH: usize>(
        seed: [u8; 32],
    ) -> Trace<timestamped::Item, SimObject> {
        let mut rng = rand_chacha::ChaCha8Rng::from_seed(seed);
        let n_parties = 10;
        let initial_triggers = super::trigger_conf_10();
        let length = 18 * SimDuration::MONTH;

        TimestampSim::sim(
            &mut rng,
            n_parties,
            initial_triggers,
            length,
            timestamped::run_protocol_dynamic_split::<_, _, _, THRESH>,
        )
    }

    #[cfg(test)]
    mod tests {
        use crate::scenarios::dynamic::TraceEntryRecord;

        #[test]
        fn run_timestamped_experiment() {
            let seed = [0u8; 32];
            let trace = super::timestamped_experiment::<3, 4>(seed);
            let mut wtr = csv::WriterBuilder::new().flexible(true).from_writer(vec![]);

            for (meta, entry) in trace.entries() {
                let rec: TraceEntryRecord<super::TimestampSim> = entry.clone().into();
                wtr.serialize((meta, rec)).unwrap();
            }
            let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
            println!("{data}");
        }
    }
}

pub mod uniform {

    use rand::SeedableRng;
    use unionize::item::le_byte_array::LEByteArray;
    use unionize::protocol::Encodable;
    use unionize::Object;

    use crate::scenarios::dynamic::{SimDuration, SimObject, Simulator, Trace};
    use crate::suites::uniform;

    impl Object<uniform::Item> for SimObject {
        fn to_item(&self) -> uniform::Item {
            let mut buf = [0u8; 30];
            let post_id_bs = self.post_id.to_le_bytes();
            let author_bs = self.author.to_le_bytes();

            for i in 0..author_bs.len() {
                buf[i] = author_bs[i];
            }

            for i in 0..post_id_bs.len() {
                buf[i + 8] = post_id_bs[i];
            }

            LEByteArray(buf)
        }

        fn validate_self_consistency(&self) -> bool {
            true
        }
    }

    use crate::scenarios::tree::mem_rc;

    #[derive(Clone)]
    pub struct UniformSim;

    impl Simulator for UniformSim {
        type Item = LEByteArray<30>;
        type Monoid = uniform::Monoid;
        type Node = uniform::Node;
        type Tree = mem_rc::Tree<Self::Monoid>;
        type EncodedMonoid = <uniform::Monoid as Encodable>::Encoded;
    }

    pub fn uniform_experiment<const SPLITS: usize, const THRESH: usize>(
        seed: [u8; 32],
    ) -> Trace<LEByteArray<30>, SimObject> {
        let mut rng = rand_chacha::ChaCha8Rng::from_seed(seed);
        let n_parties = 10;
        let initial_triggers = super::trigger_conf_10();
        let length = 18 * SimDuration::MONTH;

        UniformSim::sim(
            &mut rng,
            n_parties,
            initial_triggers,
            length,
            uniform::run_protocol::<_, _, _, SPLITS, THRESH>,
        )
    }

    #[cfg(test)]
    mod tests {
        use crate::scenarios::dynamic::TraceEntryRecord;

        #[test]
        fn run_uniform_experiment_2_3() {
            let seed = [0u8; 32];
            let trace = super::uniform_experiment::<2, 3>(seed);
            let mut wtr = csv::WriterBuilder::new().flexible(true).from_writer(vec![]);

            for (meta, entry) in trace.entries() {
                let rec: TraceEntryRecord<super::UniformSim> = entry.clone().into();
                wtr.serialize((meta, rec)).unwrap();
            }
            let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
            println!("{data}");
        }

        #[test]
        fn run_uniform_experiment_2_2() {
            let seed = [0u8; 32];
            let trace = super::uniform_experiment::<2, 2>(seed);
            let mut wtr = csv::WriterBuilder::new().flexible(true).from_writer(vec![]);

            for (meta, entry) in trace.entries() {
                let rec: TraceEntryRecord<super::UniformSim> = entry.clone().into();
                wtr.serialize((meta, rec)).unwrap();
            }
            let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
            println!("{data}");
        }

        #[test]
        fn run_uniform_experiment_3_4() {
            let seed = [0u8; 32];
            let trace = super::uniform_experiment::<3, 4>(seed);
            let mut wtr = csv::WriterBuilder::new().flexible(true).from_writer(vec![]);

            for (meta, entry) in trace.entries() {
                let rec: TraceEntryRecord<super::UniformSim> = entry.clone().into();
                wtr.serialize((meta, rec)).unwrap();
            }
            let data = String::from_utf8(wtr.into_inner().unwrap()).unwrap();
            println!("{data}");
        }
    }
}
