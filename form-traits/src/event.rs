use std::fmt::Debug;

pub trait Event: Debug {}

pub trait IntoEvent {
    type Event: Event;
    fn into_event(&self) -> Self::Event;
    fn to_inner(self: Box<Self>) -> Self::Event; 
}
