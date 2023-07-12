//! The idea is to model a system with many participants that act probabilistically.
//! The way we model this is to say "each hour the likelihood they perform action X is Y".
//! We do provide some helpers so we can say "they post about twice a day", and it gets transformed
//! into that other form.

use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use rand::RngCore;
use serde::{Deserialize, Serialize};
use unionize::{
    protocol::{Encodable, ProtocolMonoid, RespondError},
    Item, Monoid, Node, Object,
};

use super::{protocol::RunStats, tree::Tree};

#[derive(Debug, Clone)]
pub struct Probability(u64);

impl Probability {
    const MINUTE: u64 = Self::HOUR * 60;
    const HOUR: u64 = Self::DAY * 24;
    const WEEK: u64 = Self::DAY / 7;
    const DAY: u64 = Self::MONTH * 30;
    const MONTH: u64 = Self::YEAR * 12;
    const YEAR: u64 = Self::SEVEN_YEARS * 7;
    const SEVEN_YEARS: u64 = 100;

    pub fn once_per_minute() -> Probability {
        Probability(Self::MINUTE)
    }
    pub fn from_freq_per_hour(freq: u64) -> Probability {
        Probability(freq * Self::HOUR)
    }
    pub fn from_freq_per_day(freq: u64) -> Probability {
        Probability(freq * Self::DAY)
    }
    pub fn from_freq_per_week(times: u64) -> Probability {
        Probability(times * Self::WEEK)
    }
    pub fn from_freq_per_month(times: u64) -> Probability {
        Probability(times * Self::MONTH)
    }
    pub fn from_freq_per_year(times: u64) -> Probability {
        Probability(times * Self::YEAR)
    }

    pub fn does_fire(&self, roll: &DiceRoll<{ Self::MINUTE }>) -> bool {
        self.0 >= roll.0
    }
}

pub struct DiceRoll<const SIDES: u64>(u64);

impl<const SIDES: u64> DiceRoll<SIDES> {
    pub fn roll<R: RngCore>(rng: &mut R) -> Self {
        loop {
            let mut sample: u64 = rng.next_u64();
            let bits = SIDES.ilog2() + 1;
            sample = sample & ((1 << bits) - 1);
            if sample < SIDES {
                break Self(sample);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimObject {
    pub author: usize,
    pub post_id: usize,
}

#[derive(Debug, Clone)]
pub enum Event {
    Post,
    Sync(usize), // partner's party id
}

#[derive(Clone)]
pub struct SystemSpec<M, N, O, const P: usize>
where
    M: Monoid + Encodable + ProtocolMonoid,
    N: Node<M>,
    O: Object<M::Item> + for<'de2> Deserialize<'de2> + Serialize,
    M::Item: Serialize,
    M::Encoded: Serialize,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> M::Encoded: Deserialize<'de2>,
{
    probabilities: Vec<(usize, Probability, Event)>,

    run_protocol: fn(
        initiator_node: &N,
        initiator_objects: &BTreeMap<M::Item, O>,
        responder_node: &N,
        responder_objects: &BTreeMap<M::Item, O>,
    ) -> Result<(Vec<O>, Vec<O>, RunStats, RunStats), RespondError<M>>,
}

impl<M: std::fmt::Debug, N: std::fmt::Debug, O: std::fmt::Debug, const P: usize> std::fmt::Debug
    for SystemSpec<M, N, O, P>
where
    M: Monoid + Encodable + ProtocolMonoid,
    N: Node<M>,
    O: Object<M::Item> + for<'de2> Deserialize<'de2> + Serialize,
    M::Item: Serialize,
    M::Encoded: Serialize,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> M::Encoded: Deserialize<'de2>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemSpec")
            .field("probabilities", &self.probabilities)
            .field("run_protocol", &"<function>")
            .finish()
    }
}

// N: number of parties
#[derive(Debug, Clone)]
pub struct SystemState<T, M, N, O>
where
    T: Tree<M, N>,
    N: Node<M>,
    M: Monoid,
    O: Object<M::Item>,
{
    party_states: Vec<PartyState<T, M, N, O>>,
    cur_post_id: usize,
    _phantom: PhantomData<N>,
}

impl<T, M, N, O> SystemState<T, M, N, O>
where
    T: Tree<M, N>,
    N: Node<M>,
    M: Monoid,
    O: Object<M::Item>,
{
    pub fn new(n_parties: usize) -> Self {
        SystemState {
            party_states: vec![PartyState::new(); n_parties],
            cur_post_id: 0,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartyState<T, M, N, O>
where
    T: Tree<M, N>,
    N: Node<M>,
    M: Monoid,
    O: Object<M::Item>,
{
    tree: T,
    objects: BTreeMap<M::Item, O>,
    _phantom: PhantomData<N>,
}

impl<T, M, N, O> PartyState<T, M, N, O>
where
    T: Tree<M, N>,
    N: Node<M>,
    M: Monoid,
    O: Object<M::Item>,
{
    pub fn new() -> Self {
        PartyState {
            tree: T::nil(),
            objects: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }

    pub fn post(&mut self, obj: O) {
        self.tree.insert(obj.to_item());
        self.objects.insert(obj.to_item(), obj);
    }
}

pub trait ObjectBuilder {
    type Object: Object<Self::Item>;
    type Item: Item;
    fn build(sim_obj: SimObject) -> Self::Object;
}

#[derive(Debug, Clone)]
pub enum TraceEntry<I, O>
where
    I: Item,
    O: Object<I>,
{
    Posted(O),
    Sync(RunStats, RunStats),
    Phantom(I),
}

#[derive(Debug, Clone)]
pub struct Trace<I: Item, O: Object<I>>(Vec<(u64, usize, Event, TraceEntry<I, O>)>);

impl<I: Item, O: Object<I>> Trace<I, O> {
    pub fn entries(&self) -> &Vec<(u64, usize, Event, TraceEntry<I, O>)> {
        &self.0
    }
}

pub fn sim<R, T, M, N, B>(
    rng: &mut R,
    n_parties: usize,
    probabilities: Vec<(usize, Probability, Event)>,
    length: Duration,
    run_protocol: fn(
        initiator_node: &N,
        initiator_objects: &BTreeMap<M::Item, B::Object>,
        responder_node: &N,
        responder_objects: &BTreeMap<M::Item, B::Object>,
    ) -> Result<
        (Vec<B::Object>, Vec<B::Object>, RunStats, RunStats),
        RespondError<M>,
    >,
) -> Trace<M::Item, B::Object>
where
    T: Tree<M, N>,
    N: Node<M>,
    M: Monoid + unionize::protocol::ProtocolMonoid,
    R: rand::RngCore,
    B: ObjectBuilder,
    B::Object: Object<M::Item>,
    // serde stuff starts here
    M::Encoded: Serialize,
    M::Item: Serialize,
    B::Object: Serialize,
    for<'de2> M::Encoded: Deserialize<'de2>,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> B::Object: Deserialize<'de2>,
{
    let mut state = SystemState::<T, M, N, B::Object>::new(n_parties);
    let mut trace = vec![];

    for t in 0..(length.as_secs() / 60) {
        for possible_event in &probabilities {
            let roll = DiceRoll::roll(rng);
            let (party_id, prob, event) = possible_event;
            if !prob.does_fire(&roll) {
                continue;
            }

            // println!("event fired at party {party_id}: {event:?}");

            let trace_entry = match event {
                Event::Post => {
                    let sim_obj = SimObject {
                        author: *party_id,
                        post_id: state.cur_post_id,
                    };

                    let obj = B::build(sim_obj);
                    state.party_states[*party_id].post(obj.clone());
                    state.cur_post_id += 1;
                    TraceEntry::Posted(obj)
                }
                Event::Sync(partner_party_id) => {
                    let initiator_node = state.party_states[*party_id].tree.node();
                    let responder_node = state.party_states[*partner_party_id].tree.node();

                    let initiator_objects = &state.party_states[*party_id].objects;
                    let responder_objects = &state.party_states[*partner_party_id].objects;
                    let (
                        initiator_new_objects,
                        responder_new_objects,
                        initiator_stats,
                        responder_stats,
                    ) = (run_protocol)(
                        initiator_node,
                        initiator_objects,
                        responder_node,
                        responder_objects,
                    )
                    .unwrap();

                    for obj in initiator_new_objects {
                        let initiator_state = &mut state.party_states[*party_id];
                        initiator_state.tree.insert(obj.to_item());
                        initiator_state.objects.insert(obj.to_item(), obj);
                    }

                    for obj in responder_new_objects {
                        let responder_state = &mut state.party_states[*partner_party_id];
                        responder_state.tree.insert(obj.to_item());
                        responder_state.objects.insert(obj.to_item(), obj);
                    }

                    TraceEntry::Sync(initiator_stats, responder_stats)
                }
            };
            trace.push((t, *party_id, event.clone(), trace_entry));
        }
    }

    Trace(trace)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     #[test]
//     fn probabilities() {
//         let p = Probability::from_freq_per_hour(1);
//         let mut rng = rand::thread_rng();
//
//         let mut count = 0;
//
//         for t in 0..(60 * 100) {
//             let roll = DiceRoll::roll(&mut rng);
//             if p.does_fire(&roll) {
//                 count += 1;
//             }
//         }
//
//         println!("{count}");
//     }
// }
