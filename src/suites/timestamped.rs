use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use unionize::easy::timestamped::{split, split_dynamic};
use unionize::monoid::timestamped::Timestamped;
use unionize::protocol::{Encodable, ProtocolMonoid, RespondError};
use unionize::{Monoid as MonoidTrait, Node as NodeTrait, Object as ObjectTrait};

use crate::scenarios::protocol::run_protocol as run_base_protocol;

use unionize::{
    item::timestamped::TimestampedItem,
    monoid::{count::CountingMonoid, mulhash_xs233::Xsk233MulHashMonoid},
};

use crate::scenarios::dynamic::SimInstant;
use crate::scenarios::protocol::RunStats;

pub type Item = TimestampedItem<SimInstant, super::uniform::Item>;
pub type Monoid = Timestamped<SimInstant, CountingMonoid<Xsk233MulHashMonoid>>;
pub type Node = unionize::tree::mem_rc::Node<Monoid>;

pub fn run_protocol<M, N, O, const SPLIT: usize, const THRESH: usize>(
    initiator_node: &N,
    initiator_objects: &BTreeMap<M::Item, O>,
    responder_node: &N,
    responder_objects: &BTreeMap<M::Item, O>,
) -> Result<(Vec<O>, Vec<O>, RunStats, RunStats), RespondError<M>>
where
    M: MonoidTrait + Encodable + ProtocolMonoid,
    N: NodeTrait<M>,
    O: ObjectTrait<M::Item> + for<'de2> Deserialize<'de2> + Serialize,
    M::Item: Serialize,
    M::Encoded: Serialize,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> M::Encoded: Deserialize<'de2>,
{
    let res = run_base_protocol(
        initiator_node,
        initiator_objects,
        responder_node,
        responder_objects,
        THRESH,
        split::<SPLIT>,
    );

    res
}
pub fn run_protocol_dynamic_split<M, N, O, const THRESH: usize>(
    initiator_node: &N,
    initiator_objects: &BTreeMap<M::Item, O>,
    responder_node: &N,
    responder_objects: &BTreeMap<M::Item, O>,
) -> Result<(Vec<O>, Vec<O>, RunStats, RunStats), RespondError<M>>
where
    M: MonoidTrait + Encodable + ProtocolMonoid,
    N: NodeTrait<M>,
    O: ObjectTrait<M::Item> + for<'de2> Deserialize<'de2> + Serialize,
    M::Item: Serialize,
    M::Encoded: Serialize,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> M::Encoded: Deserialize<'de2>,
{
    let res = run_base_protocol(
        initiator_node,
        initiator_objects,
        responder_node,
        responder_objects,
        THRESH,
        split_dynamic::<THRESH>,
    );

    res
}
