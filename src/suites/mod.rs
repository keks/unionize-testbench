pub mod uniform {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};
    pub use unionize::easy::uniform::*;
    use unionize::protocol::{Encodable, ProtocolMonoid, RespondError};
    use unionize::{Monoid as MonoidTrait, Node as NodeTrait, Object as ObjectTrait};

    use crate::scenarios::protocol::{run_protocol as run_uniform_protocol, RunStats};

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
        let res = run_uniform_protocol(
            initiator_node,
            initiator_objects,
            responder_node,
            responder_objects,
            THRESH,
            split::<SPLIT>,
        );

        res
    }
}
pub mod timestamped;
