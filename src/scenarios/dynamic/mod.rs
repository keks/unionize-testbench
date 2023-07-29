//! We do provide some helpers so we can say "they post about twice a day", and it gets transformed
//! into that other form.
//
use std::{collections::BTreeMap, marker::PhantomData, rc::Rc};

use rand::RngCore;
use serde::{Deserialize, Serialize};
use unionize::{
    protocol::{ProtocolMonoid, RespondError},
    Item, Node, Object,
};

use super::{protocol::RunStats, tree::Tree};

pub type RunProtocolFn<S> = fn(
    initiator_node: &<S as Simulator>::Node,
    initiator_objects: &BTreeMap<<S as Simulator>::Item, SimObject>,
    responder_node: &<S as Simulator>::Node,
    responder_objects: &BTreeMap<<S as Simulator>::Item, SimObject>,
) -> Result<
    (
        Vec<SimObject>, // new objects for initiator
        Vec<SimObject>, // and responder
        RunStats,       // stats for initiator
        RunStats,       // and responder
    ),
    RespondError<<S as Simulator>::Monoid>,
>;

pub trait Simulator: Sized + Clone
where
    SimObject: Object<Self::Item>,
{
    const ITEM_SIZE: usize;
    const MONOID_SIZE: usize;

    type Item: Item + Serialize + for<'de2> Deserialize<'de2>;
    type Monoid: ProtocolMonoid<Item = Self::Item, Encoded = Self::EncodedMonoid>;
    type Node: Node<Self::Monoid>;
    type Tree: Tree<Self::Monoid, Self::Node>;
    type EncodedMonoid: Serialize + for<'de2> Deserialize<'de2>;

    fn sim<R: RngCore>(
        rng: &mut R,
        n_parties: usize,
        initial_triggers: Triggers,
        length: SimDuration,
        run_protocol: RunProtocolFn<Self>,
    ) -> Trace<Self::Item, SimObject> {
        let mut state = SystemState::<Self>::new(n_parties, initial_triggers);
        let mut trace = vec![];

        for t in 0..length.0 {
            let t = SimInstant(t);
            // eprint!(".");
            if let Some(triggers) = state.triggers.scheduled.get(&t) {
                for (party_id, event) in triggers.clone() {
                    let trace_meta = TraceMeta::new(t, party_id, &event);
                    // eprint!("x");
                    let trace_entry = state.handle_event(&event, t, party_id, run_protocol);
                    trace.push((trace_meta, trace_entry));
                }
            }

            for i in 0..state.triggers.probabilistic.len() {
                let possible_event = state.triggers.probabilistic[i].clone();
                let roll = DiceRoll::roll(rng);
                let (party_id, prob, event) = possible_event;
                if prob.does_fire(&roll) {
                    let trace_meta = TraceMeta::new(t, party_id, &event);
                    // eprint!("+");
                    let trace_entry = state.handle_event(&event, t, party_id, run_protocol);
                    trace.push((trace_meta, trace_entry));
                }
            }
        }

        Trace(trace)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SimInstant(pub u64);

impl SimInstant {
    pub fn zero() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SimDuration(pub u64);

impl std::ops::Add<SimDuration> for SimInstant {
    type Output = Self;

    fn add(self, rhs: SimDuration) -> Self::Output {
        SimInstant(self.0 + rhs.0)
    }
}

impl SimDuration {
    pub const MINUTE: Self = Self(1);
    pub const HOUR: Self = Self(60);
    pub const DAY: Self = Self(60 * 24);
    pub const WEEK: Self = Self(60 * 24 * 7);
    pub const MONTH: Self = Self(60 * 24 * 30);
    pub const YEAR: Self = Self(60 * 24 * 30 * 12);
    pub fn zero() -> Self {
        Self(0)
    }
}

impl std::ops::Mul<SimDuration> for u64 {
    type Output = SimDuration;

    fn mul(self, rhs: SimDuration) -> Self::Output {
        SimDuration(self * rhs.0)
    }
}

impl From<std::time::Duration> for SimDuration {
    fn from(value: std::time::Duration) -> Self {
        Self(value.as_secs() / 60)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frequency(pub u64);

impl Frequency {
    const PER_MINUTE: Self = Self(Self::PER_HOUR.0 * 60);
    const PER_HOUR: Self = Self(Self::PER_DAY.0 * 24);
    const PER_WEEK: Self = Self(Self::PER_DAY.0 / 7);
    const PER_DAY: Self = Self(Self::PER_MONTH.0 * 30);
    const PER_MONTH: Self = Self(Self::PER_YEAR.0 * 12);
    const PER_YEAR: Self = Self(Self::PER_SEVEN_YEARS.0 * 7);
    const PER_SEVEN_YEARS: Self = Self(100);

    pub fn once_per_minute() -> Self {
        Self(Self::PER_MINUTE.0)
    }
    pub fn from_freq_per_hour(freq: u64) -> Self {
        Self(freq * Self::PER_HOUR.0)
    }
    pub fn from_freq_per_day(freq: u64) -> Self {
        Self(freq * Self::PER_DAY.0)
    }
    pub fn from_freq_per_week(times: u64) -> Self {
        Self(times * Self::PER_WEEK.0)
    }
    pub fn from_freq_per_month(times: u64) -> Self {
        Self(times * Self::PER_MONTH.0)
    }
    pub fn from_freq_per_year(times: u64) -> Self {
        Self(times * Self::PER_YEAR.0)
    }

    pub fn from_period(period: SimDuration) -> Self {
        Self(Self::PER_MINUTE.0 / period.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Probability(u64);

impl Probability {
    const ONE: Self = Self(Frequency::PER_MINUTE.0);

    pub fn from_frequency(freq: Frequency) -> Self {
        Self(freq.0)
    }

    pub fn from_percent(percent: u64) -> Self {
        assert!(percent <= 100);
        Probability(percent * Self::ONE.0 / 100)
    }

    pub fn from_permille(permille: u64) -> Self {
        assert!(permille <= 1000);
        Probability(permille * Self::ONE.0 / 1000)
    }

    pub fn does_fire(&self, roll: &DiceRoll<{ Self::ONE.0 }>) -> bool {
        self.0 >= roll.0
    }
}

#[derive(Debug)]
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
    pub timestamp: SimInstant,
}

impl<I: Item> SimObjecty<I> for SimObject
where
    SimObject: Object<I>,
{
    fn author(&self) -> usize {
        self.author
    }

    fn post_id(&self) -> usize {
        self.post_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Event {
    Post,
    Sync(usize), // partner's party id
    DropProbabilities(#[serde(skip_serializing)] ProbabilisticEventFilterFn),
    AddProbabilities(#[serde(skip_serializing)] Vec<(usize, Probability, Event)>),
    ScheduleRelative(SimDuration, Vec<(usize, Event)>),
    Repeat(SimDuration, Box<Event>),
}

impl Event {
    pub fn drop_probabilities<F: Fn(&(usize, Probability, Event)) -> bool + 'static>(f: F) -> Self {
        Event::DropProbabilities(ProbabilisticEventFilterFn(Rc::new(f)))
    }

    pub fn repeat(period: SimDuration, event: Event) -> Self {
        Event::Repeat(period, Box::new(event))
    }
}

#[derive(Clone)]
pub struct ProbabilisticEventFilterFn(pub Rc<dyn Fn(&(usize, Probability, Event)) -> bool>);

impl PartialEq for ProbabilisticEventFilterFn {
    fn eq(&self, other: &Self) -> bool {
        true
    }
}

impl Eq for ProbabilisticEventFilterFn {}

impl std::fmt::Debug for ProbabilisticEventFilterFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<filter function>")
    }
}

impl ProbabilisticEventFilterFn {
    pub fn filter(&self, entry: &(usize, Probability, Event)) -> bool {
        self.0(entry)
    }
}

#[derive(Debug, Clone)]
pub struct Triggers {
    scheduled: BTreeMap<SimInstant, Vec<(usize, Event)>>,
    probabilistic: Vec<(usize, Probability, Event)>,
}

impl Triggers {
    pub fn new(
        scheduled: BTreeMap<SimInstant, Vec<(usize, Event)>>,
        probabilistic: Vec<(usize, Probability, Event)>,
    ) -> Self {
        Self {
            scheduled,
            probabilistic,
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        let keys: Vec<_> = other.scheduled.keys().cloned().collect();
        for key in keys {
            self.scheduled
                .entry(key)
                .or_default()
                .append(other.scheduled.entry(key).or_default());
        }
        self.probabilistic.append(&mut other.probabilistic);
    }
}

impl Default for Triggers {
    fn default() -> Self {
        Triggers {
            scheduled: BTreeMap::default(),
            probabilistic: vec![],
        }
    }
}

// N: number of parties
#[derive(Debug, Clone)]
pub struct SystemState<S: Simulator>
where
    SimObject: Object<S::Item>,
{
    triggers: Triggers,
    party_states: Vec<PartyState<S>>,
    cur_post_id: usize,
    _phantom: PhantomData<S>,
}

impl<S: Simulator> SystemState<S>
where
    SimObject: Object<S::Item>,
{
    pub fn new(n_parties: usize, initial_triggers: Triggers) -> Self {
        SystemState {
            triggers: initial_triggers,
            party_states: vec![PartyState::new(); n_parties],
            cur_post_id: 0,
            _phantom: PhantomData,
        }
    }

    pub fn handle_event(
        &mut self,
        event: &Event,
        time: SimInstant,
        party_id: usize,

        run_protocol: RunProtocolFn<S>,
    ) -> TraceEntry<S::Item, SimObject> {
        match event {
            Event::Post => {
                let obj = SimObject {
                    author: party_id,
                    post_id: self.cur_post_id,
                    timestamp: time,
                };

                self.party_states[party_id].post(obj.clone());
                self.cur_post_id += 1;
                TraceEntry::Posted(obj)
            }
            Event::Sync(partner_party_id) => {
                // eprint!("s{party_id}-{partner_party_id}");
                let initiator_node = self.party_states[party_id].tree.node();
                let responder_node = self.party_states[*partner_party_id].tree.node();

                let initiator_objects = &self.party_states[party_id].objects;
                let responder_objects = &self.party_states[*partner_party_id].objects;
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
                    let initiator_state = &mut self.party_states[party_id];
                    initiator_state.tree.insert(obj.to_item());
                    initiator_state.objects.insert(obj.to_item(), obj);
                }

                for obj in responder_new_objects {
                    let responder_state = &mut self.party_states[*partner_party_id];
                    responder_state.tree.insert(obj.to_item());
                    responder_state.objects.insert(obj.to_item(), obj);
                }

                TraceEntry::Sync(*partner_party_id, initiator_stats, responder_stats)
            }
            Event::DropProbabilities(filter) => {
                let mut new_probabilitistic = Vec::with_capacity(self.triggers.probabilistic.len());
                for prob_tuple in &self.triggers.probabilistic {
                    if !filter.filter(prob_tuple) {
                        new_probabilitistic.push(prob_tuple.clone());
                    }
                }

                let n_old_probs = self.triggers.probabilistic.len();
                let n_new_probs = new_probabilitistic.len();
                self.triggers.probabilistic = new_probabilitistic;

                TraceEntry::DropProbabilities(n_old_probs, n_new_probs)
            }
            Event::AddProbabilities(probs) => {
                self.triggers.probabilistic.extend(probs.iter().cloned());
                TraceEntry::AddProbabilities(probs.len())
            }
            Event::ScheduleRelative(t_rel, entries) => {
                for entry in entries {
                    self.triggers
                        .scheduled
                        .entry(time + *t_rel)
                        .or_default()
                        .push(entry.clone());
                }
                TraceEntry::ScheduleRelative(entries.len())
            }
            Event::Repeat(t_rel_every, inner_event) => {
                self.triggers
                    .scheduled
                    .entry(time + *t_rel_every)
                    .or_default()
                    .push((party_id, event.clone()));
                self.handle_event(inner_event, time, party_id, run_protocol)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartyState<S: Simulator>
where
    SimObject: Object<S::Item>,
{
    tree: S::Tree,
    objects: BTreeMap<S::Item, SimObject>,
    _phantom: PhantomData<S>,
}

impl<S: Simulator> PartyState<S>
where
    SimObject: Object<S::Item>,
{
    pub fn new() -> Self {
        PartyState {
            tree: S::Tree::nil(),
            objects: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }

    pub fn post(&mut self, obj: SimObject) {
        self.tree.insert(obj.to_item());
        self.objects.insert(obj.to_item(), obj);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceMeta {
    time: SimInstant,
    party_id: usize,
    event: String,
}

impl TraceMeta {
    fn new(time: SimInstant, party_id: usize, event: &Event) -> Self {
        TraceMeta {
            time,
            party_id,
            event: format!("{event:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TraceEntry<I, O>
where
    I: Item,
    O: Object<I>,
{
    Posted(O),
    Sync(usize, RunStats, RunStats),
    DropProbabilities(usize, usize),
    AddProbabilities(usize),
    ScheduleRelative(usize),
    Phantom(I),
}

#[derive(Clone, Debug, Serialize)]
pub struct TraceEntryRecord<S: Simulator>
where
    SimObject: Object<S::Item>,
{
    kind: String,
    posted_object_author: Option<usize>,
    posted_object_post_id: Option<usize>,
    sync_resp_party_id: Option<usize>,
    sync_initiator_msgs_sent: Option<usize>,
    sync_initiator_item_sets_sent: Option<usize>,
    sync_initiator_fingerprints_sent: Option<usize>,
    sync_initiator_items_sent: Option<usize>,
    sync_initiator_items_wanted: Option<usize>,
    sync_initiator_objects_sent: Option<usize>,
    sync_initiator_items_known: Option<usize>,
    sync_initiator_bytes_sent: Option<usize>,
    sync_responder_msgs_sent: Option<usize>,
    sync_responder_item_sets_sent: Option<usize>,
    sync_responder_fingerprints_sent: Option<usize>,
    sync_responder_items_sent: Option<usize>,
    sync_responder_items_wanted: Option<usize>,
    sync_responder_objects_sent: Option<usize>,
    sync_responder_items_known: Option<usize>,
    sync_responder_bytes_sent: Option<usize>,
    drop_probabilities_entries_before: Option<usize>,
    drop_probabilities_entries_after: Option<usize>,
    add_probabilities_added: Option<usize>,
    schedule_relative_added: Option<usize>,
    #[serde(skip_serializing)]
    _phantom: PhantomData<S>,
}

impl<S: Simulator> TraceEntryRecord<S>
where
    SimObject: Object<S::Item>,
{
    fn empty() -> Self {
        Self {
            kind: String::new(),
            posted_object_author: None,
            posted_object_post_id: None,
            sync_resp_party_id: None,
            sync_initiator_msgs_sent: None,
            sync_initiator_item_sets_sent: None,
            sync_initiator_fingerprints_sent: None,
            sync_initiator_items_sent: None,
            sync_initiator_items_wanted: None,
            sync_initiator_objects_sent: None,
            sync_initiator_items_known: None,
            sync_initiator_bytes_sent: None,
            sync_responder_msgs_sent: None,
            sync_responder_item_sets_sent: None,
            sync_responder_fingerprints_sent: None,
            sync_responder_items_sent: None,
            sync_responder_items_wanted: None,
            sync_responder_objects_sent: None,
            sync_responder_items_known: None,
            sync_responder_bytes_sent: None,
            drop_probabilities_entries_before: None,
            drop_probabilities_entries_after: None,
            add_probabilities_added: None,
            schedule_relative_added: None,
            _phantom: PhantomData,
        }
    }
}

pub trait SimObjecty<I: Item>: Object<I> {
    fn author(&self) -> usize;
    fn post_id(&self) -> usize;
}

impl<S: Simulator> From<TraceEntry<S::Item, SimObject>> for TraceEntryRecord<S>
where
    SimObject: Object<S::Item>,
{
    fn from(value: TraceEntry<S::Item, SimObject>) -> Self {
        let mut res = Self::empty();
        match value {
            TraceEntry::Posted(obj) => {
                res.kind = "Posted".to_string();
                res.posted_object_post_id = Some(obj.post_id());
                res.posted_object_author = Some(obj.author());
            }
            TraceEntry::Sync(resp_party_id, init, resp) => {
                res.kind = "Sync".to_string();
                res.sync_resp_party_id = Some(resp_party_id);
                res.sync_initiator_msgs_sent = Some(init.msgs_sent);
                res.sync_initiator_item_sets_sent = Some(init.item_sets_sent);
                res.sync_initiator_fingerprints_sent = Some(init.fingerprints_sent);
                res.sync_initiator_items_sent = Some(init.items_sent);
                res.sync_initiator_items_wanted = Some(init.items_wanted);
                res.sync_initiator_objects_sent = Some(init.objects_sent);
                res.sync_initiator_items_known = Some(init.items_known);
                res.sync_responder_msgs_sent = Some(resp.msgs_sent);
                res.sync_responder_item_sets_sent = Some(resp.item_sets_sent);
                res.sync_responder_fingerprints_sent = Some(resp.fingerprints_sent);
                res.sync_responder_items_sent = Some(resp.items_sent);
                res.sync_responder_items_wanted = Some(resp.items_wanted);
                res.sync_responder_objects_sent = Some(resp.objects_sent);
                res.sync_responder_items_known = Some(resp.items_known);

                res.sync_initiator_bytes_sent = Some(
                    (2 * S::ITEM_SIZE + S::MONOID_SIZE) * init.fingerprints_sent
                        + (2 * S::ITEM_SIZE) * init.item_sets_sent
                        + S::ITEM_SIZE * init.items_sent,
                );
                res.sync_responder_bytes_sent = Some(
                    90 * resp.fingerprints_sent + 60 * resp.item_sets_sent + 30 * resp.items_sent,
                );
            }
            TraceEntry::DropProbabilities(before, after) => {
                res.kind = "DropProbabilities".to_string();
                res.drop_probabilities_entries_before = Some(before);
                res.drop_probabilities_entries_after = Some(after);
            }
            TraceEntry::AddProbabilities(added) => {
                res.kind = "AddProbabilities".to_string();
                res.add_probabilities_added = Some(added);
            }
            TraceEntry::ScheduleRelative(added) => {
                res.kind = "ScheduleRelative".to_string();
                res.schedule_relative_added = Some(added)
            }
            TraceEntry::Phantom(_) => {}
        }
        res
    }
}

#[derive(Debug, Clone)]
pub struct Trace<I: Item, O: Object<I>>(Vec<(TraceMeta, TraceEntry<I, O>)>);

impl<I: Item, O: Object<I>> Trace<I, O> {
    pub fn entries(&self) -> &Vec<(TraceMeta, TraceEntry<I, O>)> {
        &self.0
    }
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
