use unionize::monoid::{count::CountingMonoid, mulhash_xs233::Xsk233MulHashMonoid};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Item<I>(u64, I)
where
    I: unionize::Item;
type Monoid = CountingMonoid<Xsk233MulHashMonoid>;
type Node = unionize::tree::mem_rc::Node<Monoid>;

impl<I: unionize::Item> unionize::Item for Item<I> {
    fn zero() -> Self {
        Item(0, I::zero())
    }

    fn next(&self) -> Self {
        Item(self.0, self.1.next())
    }
}
