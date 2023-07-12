mod uniform {
    use std::time::Duration;

    use rand::{RngCore, SeedableRng};
    use unionize::item::le_byte_array::LEByteArray;
    use unionize::Object;

    use crate::scenarios::dynamic::{self, Event, ObjectBuilder, Probability, SimObject, Trace};
    use crate::suites::uniform;

    struct UniformObjectBuilder;

    impl Object<uniform::Item> for dynamic::SimObject {
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

    impl ObjectBuilder for UniformObjectBuilder {
        type Object = dynamic::SimObject;

        type Item = LEByteArray<30>;

        fn build(sim_obj: dynamic::SimObject) -> Self::Object {
            sim_obj
        }
    }

    use crate::scenarios::tree::mem_rc;

    fn experiment(seed: [u8; 32]) -> Trace<LEByteArray<30>, SimObject> {
        let n_parties = 3;
        let probabilities = vec![
            (0, Probability::from_freq_per_day(2), Event::Post),
            (0, Probability::from_freq_per_week(3), Event::Sync(2)),
            (1, Probability::from_freq_per_day(2), Event::Post),
            (1, Probability::from_freq_per_week(3), Event::Sync(2)),
        ];
        let length = Duration::from_secs(2 * 7 * 24 * 60 * 60);
        let mut rng = rand_chacha::ChaCha12Rng::from_seed(seed);

        dynamic::sim::<_, mem_rc::Tree<uniform::Monoid>, _, _, UniformObjectBuilder>(
            &mut rng,
            n_parties,
            probabilities,
            length,
            uniform::run_protocol::<_, _, _, 2, 3>,
        )
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn run_experiment() {
            let seed = [0u8; 32];
            let trace = super::experiment(seed);
            for entry in trace.entries() {
                println!("{entry:?}");
            }
        }
    }
}
