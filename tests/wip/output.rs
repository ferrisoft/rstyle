use cheap_clone::CheapClone;
use index::Index;
use std::any::Any;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

pub use bitflags;
pub use serde;
pub use serde::Deserialize;
pub use serde::Serialize;


// ==============
// === Result ===
// ==============

type Result<T=(), E=anyhow::Error> = std::result::Result<T, E>;


// ===============
// === Message ===
// ===============

#[macro_export]
macro_rules! message {
    (
        $(#$meta:tt)*
        pub enum $enum:ident { $(
            $variant:ident
            $({ $($field:ident: $field_ty:ty),*$(,)? })?
            $(($field_ty2:ty))?
        ),*$(,)? }
    ) => {
        #[derive(Clone, Copy, Debug)]
        pub enum Tag { $($variant),* }

        $crate::bitflags::bitflags! {
            #[derive(Debug, Copy, Clone, Default)]
            pub struct Flags: u32 {
                $(const $variant = 1 << Tag::$variant as u32;)*
            }
        }

        $(#$meta)*
        #[derive(Clone, Debug)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(tag = "type")]
        pub enum $enum { $(
            $variant
            $({ $($field:$field_ty),* })?
            $(($field_ty2))?
        ),* }

        impl $crate::Message for $enum {
            type Tag = Tag;
            type Flags = Flags;

            fn tag(&self) -> Tag {
                match self {
                    $(Self::$variant {..} => Tag::$variant),*
                }
            }

            fn flag(&self) -> Flags {
                match self {
                    $(Self::$variant {..} => Flags::$variant),*
                }
            }
        }
    }
}

pub trait Message: Debug + Clone + 'static + for<'de> Deserialize<'de> {
    type Tag: Debug + Copy;
    type Flags: bitflags::Flags + Debug + Copy + Default;
    fn tag(&self) -> Self::Tag;
    fn flag(&self) -> Self::Flags;
}


// ===========
// === Bus ===
// ===========

#[derive(Debug, Default)]
pub struct Bus {
    topic_to_queue_index: HashMap<String, Index<AnyQueue>>,
    type_to_queue_index: HashMap<TypeId, Index<AnyQueue>>,
    queue_vec: Vec<AnyQueue>,
}

impl Bus {
    pub fn new_queue<M: Message>(&mut self, topic: &str) -> Queue<M> {
        let index = Index::unchecked_new(self.queue_vec.len());
        let channel: Queue<M> = Default::default();
        if self.topic_to_queue_index.contains_key(topic) {
            // WARNING
        }
        self.topic_to_queue_index.insert(topic.to_string(), index);
        self.type_to_queue_index.insert(TypeId::of::<M>(), index);
        self.queue_vec.push(channel.cheap_clone().into());
        channel
    }

    pub fn queue<M: Message>(&self) -> Option<&Queue<M>> {
        let index = self.type_to_queue_index.get(&TypeId::of::<M>())?;
        self.queue_vec[**index].queue.as_any().downcast_ref::<Queue<M>>()
    }

    pub fn queue_index(&self, topic: &str) -> Option<Index<AnyQueue>> {
        self.topic_to_queue_index.get(topic).copied()
    }

    pub fn queue_by_index(&self, index: Index<AnyQueue>) -> &AnyQueue {
        &self.queue_vec[*index]
    }
}


// ================
// === AnyQueue ===
// ================

#[derive(Clone, Debug)]
pub struct AnyQueue {
    queue: Rc<dyn AnyQueueOps>,
}

pub trait AnyQueueOps: Debug {
    fn validate_str(&self, message: &str) -> Result;
    fn send_str(&self, message: &str) -> Result;
    fn as_any(&self) -> &dyn Any;
}

impl AnyQueue {
    pub fn validate_str(&self, message: &str) -> Result {
        self.queue.validate_str(message)
    }

    pub fn send_str(&self, message: &str) -> Result {
        self.queue.send_str(message)
    }
}

impl CheapClone for AnyQueue {}

impl<M: Message> From<Queue<M>> for AnyQueue {
    fn from(queue: Queue<M>) -> Self {
        Self { queue: Rc::new(queue) }
    }
}


// =============
// === Queue ===
// =============

pub struct Queue<M> {
    data: Rc<RefCell<QueueData<M>>>
}

impl<M: Message> Queue<M> {
    #[inline(always)]
    pub fn new() -> Self { Default::default() }

    #[inline(always)]
    pub fn send(&self, message: M) {
        self.data.borrow_mut().send(message);
    }

    #[inline(always)]
    pub fn take(&self) -> Option<Vec<M>> {
        let mut data = self.data.borrow_mut();
        if data.messages.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut data.messages))
        }
    }

    #[inline(always)]
    pub fn clear(&self) {
        self.data.borrow_mut().clear();
    }

    #[inline(always)]
    pub fn for_each(&self, mut f: impl FnMut(&M)) {
        self.data.borrow().for_each(&mut f)
    }
}

impl<M: Message> AnyQueueOps for Queue<M> {
    fn validate_str(&self, message: &str) -> Result {
        serde_json::from_str::<M>(message)?;
        Ok(())
    }

    #[inline(always)]
    fn send_str(&self, message: &str) -> Result {
        self.data.borrow_mut().send_str(message)
    }

    #[inline(always)]
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<M> Default for Queue<M> {
    fn default() -> Self {
        Self { data: Default::default() }
    }
}

impl<M> CheapClone for Queue<M> {}
impl<M> Clone for Queue<M> {
    fn clone(&self) -> Self {
        Self { data: self.data.cheap_clone() }
    }
}


// =================
// === QueueData ===
// =================

pub struct QueueData<M> {
    messages: Vec<M>,
}

impl<M: Debug> Debug for Queue<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data.try_borrow() {
            Ok(data) => data.fmt(f),
            Err(_) => write!(f, "Channel {{ ... }}")
        }
    }
}

impl<M: Debug> Debug for QueueData<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Channel").field("queue", &self.messages).finish()
    }
}

impl<M> Default for QueueData<M> {
    fn default() -> Self {
        Self { messages: Default::default() }
    }
}

impl<M: Message> QueueData<M> {
    #[inline(always)]
    pub fn send(&mut self, message: M) {
        self.messages.push(message);
    }

    pub fn send_str(&mut self, message: &str) -> Result {
        let data = serde_json::from_str(message)?;
        self.send(data);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    #[inline(always)]
    pub fn for_each(&self, mut f: impl FnMut(&M)) {
        for message in &self.messages {
            f(message);
        }
    }
}


// =============
// === Tests ===
// =============

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    mod msg_a {
        use super::*;
        message! {
            pub enum MsgA {
                Ping,
                Pong,
            }
        }
    }
    use msg_a::MsgA;

    mod msg_b {
        use super::*;
        message! {
            pub enum MsgB {
                Hello,
            }
        }
    }
    use msg_b::MsgB;

    #[test]
    fn queue_by_type_returns_registered_queue() {
        let mut bus = Bus::default();
        let q = bus.new_queue::<MsgA>("a");
        q.send(MsgA::Ping);
        let looked_up = bus.queue::<MsgA>().unwrap();
        looked_up.for_each(|m| assert!(matches!(m, MsgA::Ping)));
    }

    #[test]
    fn queue_by_type_returns_none_for_unregistered() {
        let bus = Bus::default();
        assert!(bus.queue::<MsgA>().is_none());
    }

    #[test]
    fn queue_by_type_distinguishes_types() {
        let mut bus = Bus::default();
        bus.new_queue::<MsgA>("a");
        bus.new_queue::<MsgB>("b");
        assert!(bus.queue::<MsgA>().is_some());
        assert!(bus.queue::<MsgB>().is_some());
    }

    #[test]
    fn queue_by_index_still_works() {
        let mut bus = Bus::default();
        bus.new_queue::<MsgA>("a");
        let index = bus.queue_index("a").unwrap();
        let any_queue = bus.queue_by_index(index);
        assert!(any_queue.send_str(r#"{"type":"Ping"}"#).is_ok());
    }

    #[test]
    fn queue_by_type_shares_data_with_original() {
        let mut bus = Bus::default();
        let original = bus.new_queue::<MsgA>("a");
        original.send(MsgA::Pong);
        let looked_up = bus.queue::<MsgA>().unwrap();
        let msgs = looked_up.take().unwrap();
        assert_eq!(msgs.len(), 1);
        assert!(matches!(msgs[0], MsgA::Pong));
        // Original queue is now empty since they share data.
        assert!(original.take().is_none());
    }
}
