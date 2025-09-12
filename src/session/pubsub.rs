//! Publish-Subscribe event system for session management
//!
//! This module provides a pub/sub broker for distributing session events
//! throughout the application, matching the Crush architecture.

use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Event types for session operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Created,
    Updated,
    Deleted,
}

/// Event wrapper containing type and payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event<T> {
    pub event_type: EventType,
    pub payload: T,
}

impl<T> Event<T> {
    /// Create a new event
    pub fn new(event_type: EventType, payload: T) -> Self {
        Self {
            event_type,
            payload,
        }
    }
}

/// Pub/Sub broker for distributing events
pub struct Broker<T: Clone + Send + Sync + 'static> {
    subscribers: Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Event<T>>>>>,
    next_id: Arc<RwLock<usize>>,
    buffer_size: usize,
    max_events: usize,
}

impl<T: Clone + Send + Sync + 'static> Broker<T> {
    /// Create a new broker with default settings
    pub fn new() -> Self {
        Self::with_options(64, 1000)
    }
    
    /// Create a new broker with custom options
    pub fn with_options(buffer_size: usize, max_events: usize) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(0)),
            buffer_size,
            max_events,
        }
    }
    
    /// Subscribe to events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<Event<T>> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Use tokio::spawn to handle async lock
        let subscribers = self.subscribers.clone();
        let next_id = self.next_id.clone();
        
        tokio::spawn(async move {
            let mut id = next_id.write().await;
            let subscriber_id = *id;
            *id += 1;
            
            let mut subs = subscribers.write().await;
            subs.insert(subscriber_id, tx);
            
            debug!("New subscriber registered: {}", subscriber_id);
        });
        
        rx
    }
    
    /// Publish an event to all subscribers
    pub fn publish(&self, event_type: EventType, payload: T) {
        let event = Event::new(event_type, payload);
        
        // Clone for async operation
        let subscribers = self.subscribers.clone();
        let event = Arc::new(event);
        
        tokio::spawn(async move {
            let subs = subscribers.read().await;
            let mut failed_subs = Vec::new();
            
            for (id, tx) in subs.iter() {
                if let Err(_) = tx.send((*event).clone()) {
                    // Subscriber has disconnected
                    failed_subs.push(*id);
                }
            }
            
            drop(subs); // Release read lock
            
            // Clean up disconnected subscribers
            if !failed_subs.is_empty() {
                let mut subs = subscribers.write().await;
                for id in failed_subs {
                    subs.remove(&id);
                    debug!("Removed disconnected subscriber: {}", id);
                }
            }
        });
    }
    
    /// Get the current number of subscribers
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }
    
    /// Shutdown the broker and close all subscriptions
    pub async fn shutdown(&self) {
        let mut subs = self.subscribers.write().await;
        
        for (id, tx) in subs.drain() {
            drop(tx); // Close the channel
            debug!("Closed subscriber: {}", id);
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Default for Broker<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Global broker for session events
static mut SESSION_BROKER: Option<Arc<Broker<crate::session::service::ServiceSession>>> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Get or create the global session broker
pub fn session_broker() -> Arc<Broker<crate::session::service::ServiceSession>> {
    unsafe {
        INIT.call_once(|| {
            SESSION_BROKER = Some(Arc::new(Broker::new()));
        });
        SESSION_BROKER.as_ref().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone)]
    struct TestPayload {
        id: u32,
        message: String,
    }
    
    #[tokio::test]
    async fn test_broker_publish_subscribe() {
        let broker = Broker::<TestPayload>::new();
        
        // Create subscriber
        let mut rx = broker.subscribe();
        
        // Wait for subscription to be registered
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Publish event
        broker.publish(
            EventType::Created,
            TestPayload {
                id: 1,
                message: "Test".to_string(),
            },
        );
        
        // Receive event
        let event = rx.recv().await.unwrap();
        assert_eq!(event.event_type, EventType::Created);
        assert_eq!(event.payload.id, 1);
    }
    
    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker = Arc::new(Broker::<TestPayload>::new());
        
        // Create multiple subscribers
        let mut rx1 = broker.subscribe();
        let mut rx2 = broker.subscribe();
        let mut rx3 = broker.subscribe();
        
        // Wait for subscriptions
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        // Check subscriber count
        assert_eq!(broker.subscriber_count().await, 3);
        
        // Publish event
        broker.publish(
            EventType::Updated,
            TestPayload {
                id: 2,
                message: "Broadcast".to_string(),
            },
        );
        
        // All subscribers should receive the event
        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();
        let event3 = rx3.recv().await.unwrap();
        
        assert_eq!(event1.payload.id, 2);
        assert_eq!(event2.payload.id, 2);
        assert_eq!(event3.payload.id, 2);
    }
    
    #[tokio::test]
    async fn test_broker_shutdown() {
        let broker = Arc::new(Broker::<TestPayload>::new());
        
        let mut rx = broker.subscribe();
        
        // Wait for subscription
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        assert_eq!(broker.subscriber_count().await, 1);
        
        // Shutdown broker
        broker.shutdown().await;
        
        assert_eq!(broker.subscriber_count().await, 0);
        
        // Subscriber should be disconnected
        assert!(rx.recv().await.is_none());
    }
}