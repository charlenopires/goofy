//! Publish-Subscribe pattern implementation
//!
//! This module provides a generic pub/sub broker for event-driven communication
//! between components.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};

pub const BUFFER_SIZE: usize = 64;

/// Event types for pub/sub system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Created,
    Updated,
    Deleted,
    Changed,
    Selected,
    Focused,
    Blurred,
    Started,
    Stopped,
    Error,
    Warning,
    Info,
}

/// Generic event wrapper
#[derive(Debug, Clone)]
pub struct Event<T> {
    pub event_type: EventType,
    pub payload: T,
}

/// Subscriber trait for receiving events
pub trait Subscriber<T: Clone + Send + Sync + 'static> {
    /// Subscribe to events
    fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<T>>;
}

/// Publisher trait for sending events
pub trait Publisher<T: Clone + Send + Sync + 'static> {
    /// Publish an event
    fn publish(&self, event_type: EventType, payload: T);
}

/// Generic pub/sub broker
pub struct Broker<T: Clone + Send + Sync + 'static> {
    subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<Event<T>>>>>,
    max_events: usize,
}

impl<T: Clone + Send + Sync + 'static> Broker<T> {
    /// Create a new broker
    pub fn new() -> Self {
        Self::with_options(1000)
    }
    
    /// Create a new broker with custom options
    pub fn with_options(max_events: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
            max_events,
        }
    }
    
    /// Subscribe to events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<T>> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let subscribers = self.subscribers.clone();
        tokio::spawn(async move {
            subscribers.write().await.push(tx);
        });
        
        rx
    }
    
    /// Publish an event to all subscribers
    pub fn publish(&self, event_type: EventType, payload: T) {
        let event = Event {
            event_type,
            payload,
        };
        
        let subscribers = self.subscribers.clone();
        tokio::spawn(async move {
            let subs = subscribers.read().await;
            let mut closed_indices = Vec::new();
            
            for (index, subscriber) in subs.iter().enumerate() {
                if subscriber.send(event.clone()).is_err() {
                    // Subscriber has disconnected
                    closed_indices.push(index);
                }
            }
            
            // Clean up disconnected subscribers
            if !closed_indices.is_empty() {
                drop(subs);
                let mut subs = subscribers.write().await;
                for index in closed_indices.into_iter().rev() {
                    subs.remove(index);
                }
            }
        });
    }
    
    /// Get the current number of subscribers
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }
    
    /// Shutdown the broker and close all subscriber channels
    pub async fn shutdown(&self) {
        let mut subs = self.subscribers.write().await;
        subs.clear();
    }
}

impl<T: Clone + Send + Sync + 'static> Default for Broker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Send + Sync + 'static> Clone for Broker<T> {
    fn clone(&self) -> Self {
        Self {
            subscribers: Arc::clone(&self.subscribers),
            max_events: self.max_events,
        }
    }
}

/// Type-specific broker aliases
pub type SessionBroker<T> = Broker<T>;
pub type MessageBroker<T> = Broker<T>;
pub type FileBroker<T> = Broker<T>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[derive(Debug, Clone)]
    struct TestPayload {
        id: u32,
        message: String,
    }
    
    #[tokio::test]
    async fn test_pub_sub() {
        let broker = Broker::<TestPayload>::new();
        
        let mut subscriber1 = broker.subscribe();
        let mut subscriber2 = broker.subscribe();
        
        let payload = TestPayload {
            id: 1,
            message: "test".to_string(),
        };
        
        broker.publish(EventType::Created, payload.clone());
        
        // Give some time for async publish
        sleep(Duration::from_millis(10)).await;
        
        // Check subscriber 1
        if let Ok(event) = subscriber1.try_recv() {
            assert_eq!(event.event_type, EventType::Created);
            assert_eq!(event.payload.id, 1);
        } else {
            panic!("Subscriber 1 didn't receive event");
        }
        
        // Check subscriber 2
        if let Ok(event) = subscriber2.try_recv() {
            assert_eq!(event.event_type, EventType::Created);
            assert_eq!(event.payload.id, 1);
        } else {
            panic!("Subscriber 2 didn't receive event");
        }
    }
    
    #[tokio::test]
    async fn test_subscriber_count() {
        let broker = Broker::<String>::new();
        
        assert_eq!(broker.subscriber_count().await, 0);
        
        let _sub1 = broker.subscribe();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(broker.subscriber_count().await, 1);
        
        let _sub2 = broker.subscribe();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(broker.subscriber_count().await, 2);
    }
}