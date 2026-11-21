use core::{
    iter::Iterator,
    marker::{PhantomData, PhantomPinned},
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};

pub type NodePtr<T> = NonNull<Node<T>>;

#[repr(C)]
pub struct Node<T> {
    prev: Option<NodePtr<T>>,
    next: Option<NodePtr<T>>,
    pub data: T,
    _pin: PhantomPinned,
}
impl<T> Node<T> {
    pub fn new(data: T) -> Self {
        Self {
            prev: None,
            next: None,
            data,
            _pin: Default::default(),
        }
    }
    pub fn to_node_ptr(node: Pin<&mut Node<T>>) -> NodePtr<T> {
        NonNull::from(node.as_ref().get_ref())
    }
    pub fn prev(&self) -> Option<NodePtr<T>> {
        self.prev
    }
    pub fn prev_mut(&mut self) -> Option<NodePtr<T>> {
        self.prev
    }
    pub fn next(&self) -> Option<NodePtr<T>> {
        self.next
    }
    pub fn next_mut(&mut self) -> Option<NodePtr<T>> {
        self.next
    }
}
impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

pub struct UnsafeList<T> {
    front: Option<NodePtr<T>>,
    back: Option<NodePtr<T>>,
    _data: PhantomData<T>,
}

impl<T> UnsafeList<T> {
    pub const fn new() -> Self {
        Self {
            front: None,
            back: None,
            _data: PhantomData,
        }
    }

    pub fn front(&self) -> Option<NodePtr<T>> {
        self.front
    }
    pub fn front_mut(&mut self) -> Option<NodePtr<T>> {
        self.front
    }
    pub fn back(&self) -> Option<NodePtr<T>> {
        self.back
    }
    pub fn back_mut(&mut self) -> Option<NodePtr<T>> {
        self.back
    }

    /// # Safety
    /// caller must guarantee that node:
    /// - is not connected to any list.
    /// - will live until remove() is called.
    /// - while connected, its data is accessed through this list's iterators.
    /// - no reference obtained through another handle to this node's data is kept alive
    ///   while an `IterMut` item for this node is alive.
    pub unsafe fn insert_front(&mut self, node: Pin<&mut Node<T>>) -> NodePtr<T> {
        let mut node_p = Node::to_node_ptr(node);
        if let Some(front) = self.front.as_mut() {
            unsafe { front.as_mut().prev = Some(node_p) };
        }
        let node = unsafe { node_p.as_mut() };
        node.prev = None;
        node.next = self.front;
        self.front = Some(node_p);
        if self.back.is_none() {
            self.back = Some(node_p);
        }
        node_p
    }

    /// # Safety
    /// caller must guarantee that node:
    /// - is not connected to any list.
    /// - will live until remove() is called.
    /// - while connected, its data is accessed through this list's iterators.
    /// - no reference obtained through another handle to this node's data is kept alive
    ///   while an `IterMut` item for this node is alive.
    ///
    /// Also, no one must be mutating the list nodes connected to this list at the moment, such as IterMut.
    pub unsafe fn insert_back(&mut self, node: Pin<&mut Node<T>>) -> NodePtr<T> {
        let mut node_p = Node::to_node_ptr(node);
        if let Some(back) = self.back.as_mut() {
            unsafe { back.as_mut().next = Some(node_p) };
        }
        let node = unsafe { node_p.as_mut() };
        node.next = None;
        node.prev = self.back;
        self.back = Some(node_p);
        if self.front.is_none() {
            self.front = Some(node_p);
        }
        node_p
    }

    /// # Safety
    /// caller must guarantee that node:
    /// - is not connected to any list.
    /// - will live until remove() is called.
    /// - while connected, its data is accessed through this list's iterators.
    /// - no reference obtained through another handle to this node's data is kept alive
    ///   while an `IterMut` item for this node is alive.
    ///
    /// and after_p:
    /// - is a valid pointer to a node in this list, and not the same as node.
    ///
    /// Also, no one must be mutating the list nodes connected to this list at the moment, such as IterMut.
    pub unsafe fn insert_after(
        &mut self,
        mut after_p: NodePtr<T>,
        node: Pin<&mut Node<T>>,
    ) -> NodePtr<T> {
        let mut node_p = Node::to_node_ptr(node);
        assert_ne!(after_p, node_p);
        let node = unsafe { node_p.as_mut() };
        unsafe {
            node.prev = Some(after_p);
            node.next = after_p.as_ref().next;
            if let Some(mut next) = node.next {
                next.as_mut().prev = Some(node_p);
            }
            after_p.as_mut().next = Some(node_p);
        }
        if self.back.map(|p| p.addr()) == Some(after_p.addr()) {
            self.back = Some(node_p);
        }
        node_p
    }

    /// # Safety
    /// caller must guarantee that node:
    /// - is not connected to any list.
    /// - will live until remove() is called.
    /// - while connected, its data is accessed through this list's iterators.
    /// - no reference obtained through another handle to this node's data is kept alive
    ///   while an `IterMut` item for this node is alive.
    ///
    /// and before_p:
    /// - is a valid pointer to a node in this list, and not the same as node.
    ///
    /// Also, no one must be mutating the list nodes connected to this list at the moment, such as IterMut.
    pub unsafe fn insert_before(
        &mut self,
        mut before_p: NodePtr<T>,
        node: Pin<&mut Node<T>>,
    ) -> NodePtr<T> {
        let mut node_p = Node::to_node_ptr(node);
        assert_ne!(before_p, node_p);
        let node = unsafe { node_p.as_mut() };
        let before = unsafe { before_p.as_mut() };
        node.next = Some(before_p);
        node.prev = before.prev;
        if let Some(mut prev) = node.prev {
            unsafe { prev.as_mut() }.next = Some(node_p);
        }
        before.prev = Some(node_p);
        if self.front.map(|p| p.addr()) == Some(before_p.addr()) {
            self.front = Some(node_p);
        }
        node_p
    }

    /// # Safety
    /// caller must guarantee that node:
    /// - is valid pointer to node connected to this list.
    /// - no reference to this node or its data is live while its links are modified.
    ///
    /// Also, no one must be mutating the list nodes connected to this list at the moment, such as IterMut.
    pub unsafe fn remove(&mut self, mut node_p: NodePtr<T>) {
        let node = unsafe { node_p.as_mut() };
        if let Some(mut prev) = node.prev {
            unsafe { prev.as_mut() }.next = node.next;
        }
        if let Some(mut next) = node.next {
            unsafe { next.as_mut() }.prev = node.prev;
        }
        if self.front.map(|p| p.addr()) == Some(node_p.addr()) {
            self.front = node.next;
        }
        if self.back.map(|p| p.addr()) == Some(node_p.addr()) {
            self.back = node.prev;
        }
        debug_assert_eq!(self.front.is_none(), self.back.is_none());
        node.prev = None;
        node.next = None;
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            next_item: self.front,
            _list: PhantomData,
        }
    }
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            next_item: self.front,
            _list: PhantomData,
        }
    }
}
impl<T> Default for UnsafeList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a UnsafeList<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut UnsafeList<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct Iter<'a, T> {
    next_item: Option<NodePtr<T>>,
    _list: PhantomData<&'a UnsafeList<T>>,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next_item?;
        self.next_item = unsafe { current.as_ref().next() };
        Some(unsafe { &current.as_ref().data })
    }
}
pub struct IterMut<'a, T> {
    next_item: Option<NodePtr<T>>,
    _list: PhantomData<&'a mut UnsafeList<T>>,
}
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = self.next_item?;
        self.next_item = unsafe { current.as_ref().next() };
        Some(unsafe { &mut current.as_mut().data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::pin::pin;

    fn values(list: &UnsafeList<i32>) -> alloc::vec::Vec<i32> {
        let mut result = alloc::vec::Vec::new();
        let mut current = list.front;
        let mut previous = None;

        while let Some(node_p) = current {
            // Detect circular or incorrect links
            // NOTE: This limit should be increased if we create more nodes in tests.
            assert!(result.len() < 256);

            let node = unsafe { node_p.as_ref() };

            assert_eq!(node.prev, previous);
            result.push(node.data);

            previous = Some(node_p);
            current = node.next;
        }

        assert_eq!(previous, list.back);

        // Verify reverse links
        let mut current = list.back;
        let mut next = None;

        for expected in result.iter().rev() {
            let node_p = current.expect("broken backward link");
            let node = unsafe { node_p.as_ref() };

            assert_eq!(node.next, next);
            assert_eq!(&node.data, expected);

            next = Some(node_p);
            current = node.prev;
        }

        assert!(current.is_none());
        assert_eq!(next, list.front);

        result
    }

    #[test]
    fn test_empty_list() {
        let list = UnsafeList::<i32>::default();

        assert!(list.front.is_none());
        assert!(list.back.is_none());
        assert_eq!(values(&list), []);
    }

    #[test]
    fn test_insert_front() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_front(a.as_mut());
            assert_eq!(values(&list), [10]);

            list.insert_front(b.as_mut());
            assert_eq!(values(&list), [20, 10]);

            list.insert_front(c.as_mut());
            assert_eq!(values(&list), [30, 20, 10]);
        }
    }

    #[test]
    fn test_insert_back() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            assert_eq!(values(&list), [10]);

            list.insert_back(b.as_mut());
            assert_eq!(values(&list), [10, 20]);

            list.insert_back(c.as_mut());
            assert_eq!(values(&list), [10, 20, 30]);
        }
    }

    #[test]
    fn test_insert_after() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));
        let mut d = pin!(Node::new(40));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(c.as_mut());

            let a_ptr = Node::to_node_ptr(a.as_mut());
            list.insert_after(a_ptr, b.as_mut());
            assert_eq!(values(&list), [10, 20, 30]);

            let c_ptr = Node::to_node_ptr(c.as_mut());
            list.insert_after(c_ptr, d.as_mut());
            assert_eq!(values(&list), [10, 20, 30, 40]);
        }
    }

    #[test]
    fn test_insert_before() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));
        let mut d = pin!(Node::new(5));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(c.as_mut());

            let c_ptr = Node::to_node_ptr(c.as_mut());
            list.insert_before(c_ptr, b.as_mut());
            assert_eq!(values(&list), [10, 20, 30]);

            let a_ptr = Node::to_node_ptr(a.as_mut());
            list.insert_before(a_ptr, d.as_mut());
            assert_eq!(values(&list), [5, 10, 20, 30]);
        }
    }

    #[test]
    fn test_remove_front() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());

            list.remove(Node::to_node_ptr(a.as_mut()));
            assert_eq!(values(&list), [20, 30]);

            list.remove(Node::to_node_ptr(b.as_mut()));
            assert_eq!(values(&list), [30]);

            list.remove(Node::to_node_ptr(c.as_mut()));
            assert_eq!(values(&list), []);
        }
    }

    #[test]
    fn test_remove_back() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());

            list.remove(Node::to_node_ptr(c.as_mut()));
            assert_eq!(values(&list), [10, 20]);

            list.remove(Node::to_node_ptr(b.as_mut()));
            assert_eq!(values(&list), [10]);

            list.remove(Node::to_node_ptr(a.as_mut()));
            assert_eq!(values(&list), []);
        }
    }

    #[test]
    fn test_remove_middle() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));
        let mut d = pin!(Node::new(40));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());
            list.insert_back(d.as_mut());

            list.remove(Node::to_node_ptr(b.as_mut()));
            assert_eq!(values(&list), [10, 30, 40]);

            list.remove(Node::to_node_ptr(c.as_mut()));
            assert_eq!(values(&list), [10, 40]);
        }
    }

    #[test]
    fn test_remove_only_node() {
        let mut list = UnsafeList::<i32>::default();
        let mut a = pin!(Node::new(123));

        unsafe {
            list.insert_back(a.as_mut());
            assert_eq!(values(&list), [123]);

            list.remove(Node::to_node_ptr(a.as_mut()));
            assert_eq!(values(&list), []);
        }
    }

    #[test]
    fn test_reinsert_removed_node() {
        let mut list = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());

            list.remove(Node::to_node_ptr(b.as_mut()));
            assert_eq!(values(&list), [10, 30]);

            list.insert_front(b.as_mut());
            assert_eq!(values(&list), [20, 10, 30]);
        }
    }

    #[test]
    fn test_move_node_between_lists() {
        let mut list1 = UnsafeList::<i32>::default();
        let mut list2 = UnsafeList::<i32>::default();

        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));

        unsafe {
            list1.insert_back(a.as_mut());
            list1.insert_back(b.as_mut());

            list1.remove(Node::to_node_ptr(a.as_mut()));
            list2.insert_back(a.as_mut());

            assert_eq!(values(&list1), [20]);
            assert_eq!(values(&list2), [10]);
        }
    }

    #[test]
    fn test_iter_visits_nodes_in_list_order() {
        let mut list = UnsafeList::<i32>::default();
        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());
        }

        assert_eq!(
            list.iter().copied().collect::<alloc::vec::Vec<_>>(),
            [10, 20, 30]
        );
        assert!(list.iter().next().is_some());
    }

    #[test]
    fn test_iter_mut_yields_disjoint_mutable_items() {
        let mut list = UnsafeList::<i32>::default();
        let mut a = pin!(Node::new(10));
        let mut b = pin!(Node::new(20));
        let mut c = pin!(Node::new(30));

        unsafe {
            list.insert_back(a.as_mut());
            list.insert_back(b.as_mut());
            list.insert_back(c.as_mut());
        }

        // Keep earlier mutable items live while the iterator advances.
        let mut items: alloc::vec::Vec<&mut i32> = list.iter_mut().collect();
        assert_eq!(items.len(), 3);
        *items[0] = 11;
        *items[1] = 21;
        *items[2] = 31;
        drop(items);

        assert_eq!(
            list.iter().copied().collect::<alloc::vec::Vec<_>>(),
            [11, 21, 31]
        );
    }

    #[test]
    fn test_iterators_on_empty_list() {
        let mut list = UnsafeList::<i32>::default();

        assert!(list.iter().next().is_none());
        assert!(list.iter_mut().next().is_none());
    }

    #[test]
    fn test_node_deref() {
        let mut node = Node::new(42);

        assert_eq!(*node, 42);

        *node = 100;

        assert_eq!(*node, 100);
    }

    #[test]
    fn test_large_list() {
        const COUNT: usize = 128;

        // Keep each node's address stably using a Box
        let mut nodes: alloc::vec::Vec<core::pin::Pin<alloc::boxed::Box<Node<i32>>>> = (0..COUNT)
            .map(|i| alloc::boxed::Box::pin(Node::new(i as i32)))
            .collect();

        let mut list = UnsafeList::<i32>::default();

        unsafe {
            for node in nodes.iter_mut() {
                list.insert_back(node.as_mut());
            }

            assert_eq!(values(&list).len(), COUNT);

            for i in (0..COUNT).step_by(2) {
                let ptr = Node::to_node_ptr(nodes[i].as_mut());
                list.remove(ptr);
            }

            let expected: alloc::vec::Vec<i32> =
                (0..COUNT).step_by(2).map(|i| (i + 1) as i32).collect();

            assert_eq!(values(&list), expected);
        }
    }
}
