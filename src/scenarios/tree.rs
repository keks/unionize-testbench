use unionize::{Monoid, Node};

pub trait Tree<M: Monoid, N: Node<M>>: Clone + std::fmt::Debug {
    fn nil() -> Self;
    fn insert(&mut self, item: M::Item);
    fn node(&self) -> &N;
}

pub mod mem_rc {
    use unionize::{tree::mem_rc::Node, Monoid};

    #[derive(Clone, Debug)]
    pub struct Tree<M: Monoid>(Node<M>);

    impl<M: Monoid> super::Tree<M, Node<M>> for Tree<M> {
        fn nil() -> Self {
            Self(Node::nil())
        }

        fn insert(&mut self, item: <M as Monoid>::Item) {
            self.0 = self.0.insert(item)
        }

        fn node(&self) -> &Node<M> {
            &self.0
        }
    }
}

pub mod mem_rc_bounds {
    use unionize::{tree::mem_rc_bounds::Node, Monoid};

    #[derive(Clone, Debug)]
    struct Tree<M: Monoid>(Node<M>);

    impl<M: Monoid> super::Tree<M, Node<M>> for Tree<M> {
        fn nil() -> Self {
            Self(Node::nil())
        }

        fn insert(&mut self, item: <M as Monoid>::Item) {
            self.0 = self.0.insert(item)
        }

        fn node(&self) -> &Node<M> {
            &self.0
        }
    }
}
