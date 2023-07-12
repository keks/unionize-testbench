extern crate alloc;
use alloc::{vec, vec::Vec};

extern crate std;
use std::{collections::BTreeMap, io::Write, print, println};

use unionize::{
    easy::uniform::{split as uniform_split, Item as UniformItem},
    protocol::{first_message, respond_to_message, Encodable, Message, ProtocolMonoid},
    Node,
};

use crate::scenarios::tree::Tree;

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

fn sync_10k_msgs<M, N, T>()
where
    M: ProtocolMonoid<Item = UniformItem>,
    N: Node<M>,
    T: Tree<M, N>,
    <M as Encodable>::Encoded: serde::Serialize,
    for<'de2> <M as Encodable>::Encoded: serde::Deserialize<'de2>,
{
    let mut shared_msgs = vec![M::Item::default(); 6_000];
    let mut alices_msgs = vec![M::Item::default(); 2_000];
    let mut bobs_msgs = vec![M::Item::default(); 2_000];

    let mut alice_tree = T::nil();
    let mut alice_object_store = BTreeMap::new();
    let mut bob_tree = T::nil();
    let mut bob_object_store = BTreeMap::new();

    // let statm = procinfo::pid::statm_self().unwrap();
    // println!("current memory usage: {statm:#?}");

    let gen_start_time = std::time::Instant::now();

    print!("generating and adding items... ");
    std::io::stdout().flush().unwrap();
    let mut rng = ChaCha8Rng::from_seed([23u8; 32]);
    for msg in &mut shared_msgs {
        rng.fill(&mut msg.0);
        alice_tree.insert(msg.clone());
        alice_object_store.insert(msg.clone(), (msg.clone(), true));
        bob_tree.insert(msg.clone());
        bob_object_store.insert(msg.clone(), (msg.clone(), true));
    }
    for msg in &mut alices_msgs {
        rng.fill(&mut msg.0);
        alice_tree.insert(msg.clone());
        alice_object_store.insert(msg.clone(), (msg.clone(), true));
    }
    for msg in &mut bobs_msgs {
        rng.fill(&mut msg.0);
        bob_tree.insert(msg.clone());
        bob_object_store.insert(msg.clone(), (msg.clone(), true));
    }
    println!("done after {:?}.", gen_start_time.elapsed());

    // let statm = procinfo::pid::statm_self().unwrap();
    // println!("current memory usage: {statm:#?}");

    let mut msg: Message<_, (UniformItem, bool)> = first_message(alice_tree.node()).unwrap();

    let mut missing_items_alice = vec![];
    let mut missing_items_bob = vec![];

    let mut count = 0;

    let loop_start_time = std::time::Instant::now();
    loop {
        count += 1;
        println!(
            "alice msg lengths: fps:{} item_sets:{}",
            msg.fingerprints().len(),
            msg.item_sets().len()
        );
        if msg.is_end() {
            break;
        }

        let (resp, new_objects) = respond_to_message(
            bob_tree.node(),
            &bob_object_store,
            &msg,
            3,
            uniform_split::<2>,
        )
        .unwrap();
        missing_items_bob.extend(new_objects.into_iter().map(|(item, _)| item));

        println!(
            "bob msg lengths: fps:{} item_sets:{}",
            resp.fingerprints().len(),
            resp.item_sets().len()
        );
        if resp.is_end() {
            break;
        }

        let (resp, new_items) = respond_to_message(
            alice_tree.node(),
            &alice_object_store,
            &resp,
            3,
            uniform_split::<2>,
        )
        .unwrap();
        missing_items_alice.extend(new_items.into_iter().map(|(item, _)| item));

        msg = resp;
    }

    println!(
        "protocol took {count} rounds and {:?}.",
        loop_start_time.elapsed()
    );

    println!("alice: # missing items: {}", missing_items_alice.len());
    println!("bob:   # missing items: {}", missing_items_bob.len());

    let mut all_items = shared_msgs.clone();
    all_items.extend(alices_msgs.iter());
    all_items.extend(bobs_msgs.iter());

    let mut all_items_alice = shared_msgs.clone();
    all_items_alice.extend(alices_msgs.iter());

    let mut all_items_bob = shared_msgs.clone();
    all_items_bob.extend(bobs_msgs.iter());

    all_items_alice.extend(missing_items_alice.iter());
    all_items_bob.extend(missing_items_bob.iter());

    all_items.sort();
    all_items_alice.sort();
    all_items_bob.sort();

    let all_len = all_items.len();
    let alice_all_len = all_items_alice.len();
    let bob_all_len = all_items_bob.len();

    println!("lens: all:{all_len} alice:{alice_all_len}, bob:{bob_all_len}");
    assert_eq!(all_len, alice_all_len);
    assert_eq!(all_len, bob_all_len);

    let mut all: Vec<_> = Vec::from_iter(all_items.iter().cloned());
    let mut alice_all: Vec<_> = Vec::from_iter(all_items_alice.iter().cloned());
    let mut bob_all: Vec<_> = Vec::from_iter(all_items_bob.iter().cloned());

    alice_all.sort();
    bob_all.sort();
    all.sort();

    let alice_eq = alice_all == all;
    let bob_eq = bob_all == all;

    println!("{alice_eq}, {bob_eq}");
    assert!(alice_eq, "a does not match");
    assert!(bob_eq, "a does not match");
}
