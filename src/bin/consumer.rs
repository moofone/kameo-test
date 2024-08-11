use std::time::Duration;

use anyhow::Result;
use kameo::actor::UnboundedMailbox;
use kameo::spawn;
use kameo::Actor;
use kameo::{
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message::{Context, Message},
};
use kameo_test::backend::pplns::{PplnsActor, Shutdown};
use log::{error, info};
use serde::ser::StdError;
use tokio::{signal, time::interval};

#[derive(Clone)]
pub struct ConsumerActor {
    pplns_actor: ActorRef<PplnsActor>,
}

impl Actor for ConsumerActor {
    type Mailbox = UnboundedMailbox<Self>;

    fn name() -> &'static str {
        "ConsumerActor"
    }

    async fn on_start(
        &mut self,
        ctx: ActorRef<Self>,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info!("ConsumerActor started. Linking PPLNS actor as child.");
        ctx.link_child(&self.pplns_actor).await;
        Ok(())
    }

    async fn on_stop(
        self,
        ctx: WeakActorRef<Self>,
        reason: ActorStopReason,
    ) -> Result<(), Box<dyn StdError + Send + Sync>> {
        info!("ConsumerActor stopped: {:?}", reason);
        Ok(())
    }

    async fn on_link_died(
        &mut self,
        actor_ref: WeakActorRef<Self>,
        id: u64,
        reason: ActorStopReason,
    ) -> Result<Option<ActorStopReason>, Box<dyn std::error::Error + Send + Sync>> {
        info!("on_link_died called with id: {}, reason: {:?}", id, reason);
        info!("PPLNS actor with ID {} died: {:?}", id, reason);
        self.respawn_pplns_actor();
        if let Some(actor_ref) = actor_ref.upgrade() {
            actor_ref.link_child(&self.pplns_actor).await;
            info!("Linked new PPLNS actor as child");
        }
        Ok(None)
    }
}

pub struct Run;

impl Message<Run> for ConsumerActor {
    type Reply = Result<()>;

    async fn handle(&mut self, _: Run, _ctx: Context<'_, Self, Self::Reply>) -> Self::Reply {
        self.run().await
    }
}

impl ConsumerActor {
    pub fn new() -> Self {
        let pplns_actor = PplnsActor::new();
        let pplns_actor_ref = spawn(pplns_actor);

        Self {
            pplns_actor: pplns_actor_ref,
        }
    }

    pub async fn run(&self) -> Result<()> {
        let mut shutdown_interval = interval(Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = shutdown_interval.tick() => {
                    info!("Sending shutdown signal to PPLNS actor");
                    if self.pplns_actor.is_alive() {
                        if let Err(e) = self.pplns_actor.tell(Shutdown).send() {
                            error!("Failed to send shutdown signal to PPLNS actor: {:?}", e);
                        }
                    } else {
                        info!("PPLNS actor is already not alive");
                    }
                }
            }
        }
    }

    fn respawn_pplns_actor(&mut self) {
        info!("Respawning PPLNS actor");
        let new_pplns_actor = PplnsActor::new();
        let new_pplns_actor_ref = spawn(new_pplns_actor);
        let new_pplns_actor_id = new_pplns_actor_ref.id();

        info!("New PPLNS actor spawned with ID: {}", new_pplns_actor_id);
        self.pplns_actor = new_pplns_actor_ref;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Create ConsumerActor
    let consumer_actor = ConsumerActor::new();
    let consumer_actor_ref = kameo::spawn(consumer_actor);

    // Clone the actor reference for the run task
    let run_actor_ref = consumer_actor_ref.clone();

    // Run the consumer actor
    let run_handle = tokio::spawn(async move {
        if let Err(e) = run_actor_ref.ask(Run).send().await {
            error!("Error running ConsumerActor: {:?}", e);
        }
    });

    // Wait for interrupt signal
    // signal::ctrl_c().await?;
    // info!("Received interrupt signal, shutting down");

    // Stop the consumer actor gracefully
    // consumer_actor_ref.stop_gracefully().await?;

    // Wait for the run handle to complete
    run_handle.await?;

    info!("Shutting down consumer");
    Ok(())
}
