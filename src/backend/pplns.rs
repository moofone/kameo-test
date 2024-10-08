use anyhow::Result;
use kameo::actor::{ActorRef, UnboundedMailbox, WeakActorRef};
use kameo::error::ActorStopReason;
use kameo::{
    message::{Context, Message},
    Actor,
};
use log::{error, info, warn};
use serde::ser::StdError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;

// Define the actor state
pub struct PplnsActor;

impl PplnsActor {
    pub fn new() -> Self {
        PplnsActor
    }
}

impl Actor for PplnsActor {
    type Mailbox = UnboundedMailbox<Self>;

    fn name() -> &'static str {
        "PplnsActor"
    }

    async fn on_start(
        &mut self,
        ctx: ActorRef<Self>,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info!("PplnsActor started with ID: {}", ctx.id());
        Ok(())
    }

    async fn on_stop(
        self,
        ctx: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info!(
            "PplnsActor on_stop called with ID: {}, reason: {:?}",
            ctx.id(),
            reason
        );
        if let Some(actor_ref) = ctx.upgrade() {
            let links = actor_ref.as_ref().lock().await;
            info!("PplnsActor has {} links", links.len());
            for (id, _) in links.iter() {
                info!("PplnsActor is linked to actor with ID: {}", id);
            }
        } else {
            warn!("Failed to upgrade WeakActorRef in PplnsActor on_stop");
        }
        Ok(())
    }
}

// Define Messages
pub struct Shutdown;

impl Message<Shutdown> for PplnsActor {
    type Reply = ();

    async fn handle(&mut self, _: Shutdown, ctx: Context<'_, Self, Self::Reply>) -> Self::Reply {
        info!("PplnsActor received shutdown signal");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        info!("PplnsActor calling kill() on itself");
        ctx.actor_ref().kill();
        info!("PplnsActor kill() called");
    }
}
