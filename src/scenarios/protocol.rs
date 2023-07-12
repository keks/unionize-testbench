use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use unionize::{
    protocol::{
        first_message, respond_to_message, Encodable, Message, ProtocolMonoid, RespondError,
    },
    Monoid, Node, Object,
};

#[derive(Clone, Debug)]
pub struct RunStats {
    party_id: usize,
    msgs_sent: usize,
    item_sets_sent: usize,
    fingerprints_sent: usize,
    items_sent: usize,
    items_wanted: usize,
    objects_sent: usize,
}

impl RunStats {
    fn new(party_id: usize) -> Self {
        RunStats {
            party_id,
            msgs_sent: 0,
            item_sets_sent: 0,
            fingerprints_sent: 0,
            items_sent: 0,
            items_wanted: 0,
            objects_sent: 0,
        }
    }
    fn consume<M, O>(&mut self, msg: &Message<M, O>)
    where
        M: ProtocolMonoid,
        O: Object<M::Item> + Serialize + for<'de2> serde::Deserialize<'de2>,
        <M as unionize::Monoid>::Item: Serialize,
        <M as Encodable>::Encoded: Serialize,
        for<'de2> <M as unionize::Monoid>::Item: Deserialize<'de2>,
        for<'de2> <M as Encodable>::Encoded: Deserialize<'de2>,
    {
        // println!("party {} consumes message {msg:#?}", self.party_id);
        self.msgs_sent += 1;
        self.item_sets_sent += msg.item_sets().len();
        self.fingerprints_sent += msg.fingerprints().len();
        self.items_sent += msg
            .item_sets()
            .iter()
            .fold(0, |acc, set| acc + set.items().len());
        self.items_wanted += msg.wants().len();
        self.objects_sent += msg.provide().len();
    }
}

pub fn run_protocol<M, N, O>(
    initiator_node: &N,
    initiator_objects: &BTreeMap<M::Item, O>,
    responder_node: &N,
    responder_objects: &BTreeMap<M::Item, O>,
    threshold: usize,
    split: fn(usize) -> Vec<usize>,
) -> Result<(Vec<O>, Vec<O>, RunStats, RunStats), RespondError<M>>
where
    M: Monoid + Encodable + ProtocolMonoid,
    N: Node<M>,
    O: Object<M::Item> + for<'de2> Deserialize<'de2> + Serialize,
    M::Item: Serialize,
    M::Encoded: Serialize,
    for<'de2> M::Item: Deserialize<'de2>,
    for<'de2> M::Encoded: Deserialize<'de2>,
{
    let mut new_objects_initiator = vec![];
    let mut new_objects_responder = vec![];

    let mut stats_initiator = RunStats::new(0);
    let mut stats_responder = RunStats::new(1);

    let mut msg = first_message(initiator_node)?;
    stats_initiator.consume(&msg);

    loop {
        let (resp, mut new_objs) =
            respond_to_message(responder_node, responder_objects, &msg, threshold, split)?;
        msg = resp;
        stats_responder.consume(&msg);
        new_objects_responder.append(&mut new_objs);
        if msg.is_end() {
            break;
        }

        let (resp, mut new_objs) =
            respond_to_message(initiator_node, initiator_objects, &msg, threshold, split)?;
        msg = resp;
        stats_initiator.consume(&msg);
        new_objects_initiator.append(&mut new_objs);
        if msg.is_end() {
            break;
        }
    }

    Ok((
        new_objects_initiator,
        new_objects_responder,
        stats_initiator,
        stats_responder,
    ))
}
